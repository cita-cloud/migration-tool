use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;

use fs_extra::dir::copy as copy_dir;
use fs_extra::dir::CopyOptions;

use anyhow::ensure;
use anyhow::Context;
use anyhow::Result;
use serde::de::DeserializeOwned;

use crate::cert::{generate_certs, CertAndKey};

mod old {
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct ConsensusConfig {
        pub controller_port: u16,
    }

    #[derive(Deserialize)]
    pub struct ControllerConfig {
        pub network_port: u16,
        pub consensus_port: u16,
        pub storage_port: u16,
        pub kms_port: u16,
        pub executor_port: u16,
    }

    #[derive(Deserialize)]
    pub struct NetworkConfig {
        pub port: u16,
        pub peers: Vec<PeerConfig>,
    }

    #[derive(Deserialize)]
    pub struct PeerConfig {
        pub ip: String,
        pub port: u16,
    }

    #[derive(Deserialize)]
    pub struct InitSysConfig {
        pub version: u64,
        pub admin: String,
        pub block_interval: u64,
        pub chain_id: String,
        pub validators: Vec<String>,
    }

    #[derive(Deserialize)]
    pub struct Genesis {
        pub timestamp: u64,
        pub prevhash: String,
    }
}

mod new {
    use serde::Serialize;

    pub const DEFAULT_BLOCK_LIMIT: u64 = 100;
    pub const DEFAULT_PACKAGE_LIMIT: u64 = 30000;

    #[derive(Serialize)]
    pub struct ControllerConfig {
        pub consensus_port: u16,
        pub controller_port: u16,
        pub executor_port: u16,
        pub storage_port: u16,
        pub kms_port: u16,
        pub network_port: u16,

        pub key_id: u64,
        pub node_address: String,
        pub package_limit: u64,
    }

    #[derive(Serialize)]
    pub struct ConsensusRaftConfig {
        pub controller_port: u16,
        pub grpc_listen_port: u16,
        pub network_port: u16,
        pub node_addr: String,
    }

    #[derive(Serialize, Clone)]
    pub struct GenesisBlock {
        pub prevhash: String,
        pub timestamp: u64,
    }

    #[derive(Serialize, Clone)]
    pub struct SystemConfig {
        pub admin: String,
        pub block_interval: u64,
        pub block_limit: u64,
        pub chain_id: String,
        pub version: u64,
        pub validators: Vec<String>,
    }

    #[derive(Serialize)]
    pub struct NetworkTlsConfig {
        // Optional fields will be filled latter
        pub ca_cert: Option<String>,
        pub cert: Option<String>,
        pub grpc_port: u16,
        pub listen_port: u16,
        pub peers: Vec<NetworkTlsPeerConfig>,
    }

    #[derive(Serialize, Clone)]
    pub struct NetworkTlsPeerConfig {
        // Will be filled latter
        pub domain: Option<String>,
        pub host: String,
        pub port: u16,
    }

    #[derive(Serialize)]
    pub struct KmsSmConfig {
        pub kms_port: u16,
        pub db_key: String,
    }

    #[derive(Serialize)]
    pub struct StorageRocksDbConfig {
        pub kms_port: u16,
        pub storage_port: u16,
    }

    #[derive(Serialize)]
    pub struct ExecutorEvmConfig {
        pub executor_port: u16,
    }

    #[derive(Serialize)]
    pub struct Config {
        pub system_config: SystemConfig,
        pub genesis_block: GenesisBlock,

        #[serde(rename = "controller")]
        pub controller: ControllerConfig,
        #[serde(rename = "consensus_raft")]
        pub consensus: ConsensusRaftConfig,
        #[serde(rename = "storage_rocksdb")]
        pub storage: StorageRocksDbConfig,
        #[serde(rename = "executor_evm")]
        pub executor: ExecutorEvmConfig,
        #[serde(rename = "kms_sm")]
        pub kms: KmsSmConfig,
        #[serde(rename = "network_tls")]
        pub network: NetworkTlsConfig,

        // Helper data, will be filled later
        #[serde(skip)]
        pub network_host: Option<String>,
        #[serde(skip)]
        pub network_port: Option<u16>,
    }

    #[derive(Serialize)]
    pub struct MetaConfig {
        #[serde(rename = "network_tls")]
        pub network: MetaNetworkConfig,

        pub genesis_block: GenesisBlock,
        pub system_config: SystemConfig,

        pub admin_config: MetaAdminConfig,

        pub current_config: MetaCurrentConfig,
    }

    #[derive(Serialize)]
    pub struct MetaAdminConfig {
        pub admin_address: String,
        pub key_id: u64,
    }

    #[derive(Serialize)]
    pub struct MetaCurrentConfig {
        pub addresses: Vec<String>,

        pub ca_cert_pem: String,
        pub ca_key_pem: String,

        pub count: u64,

        pub ips: Vec<String>,
        pub p2p_ports: Vec<u16>,
        pub rpc_ports: Vec<u16>,

        // Always false
        pub use_num: bool,

        pub tls_peers: MetaNetworkConfig,
    }

    #[derive(Serialize, Clone)]
    pub struct MetaNetworkConfig {
        pub peers: Vec<NetworkTlsPeerConfig>,
    }
}

struct NodeConfigMigrate {
    // node config loaded from old

    // ports
    controller_port: u16,
    consensus_port: u16,
    executor_port: u16,
    network_port: u16,
    kms_port: u16,
    storage_port: u16,

    // controller
    node_addr: String,
    genesis_block: old::Genesis,
    system_config: old::InitSysConfig,

    // kms
    kms_password: String,
    key_id: u64,

    // network
    network_config: old::NetworkConfig,
}

impl NodeConfigMigrate {
    pub fn from_old(data_dir: impl AsRef<Path>) -> Result<new::Config> {
        let old =
            Self::extract_from(data_dir).context("cannot extract info from old node config")?;
        Ok(old.generate_new())
    }

    fn extract_from(data_dir: impl AsRef<Path>) -> Result<Self> {
        let old::ControllerConfig {
            consensus_port,
            storage_port,
            network_port,
            executor_port,
            kms_port,
        } = extract_toml(&data_dir, "controller-config.toml")?;

        let old::ConsensusConfig { controller_port } =
            extract_toml(&data_dir, "consensus-config.toml")?;

        let network_config: old::NetworkConfig = extract_toml(&data_dir, "network-config.toml")?;
        let node_addr = extract_text(&data_dir, "node_address")?;

        let system_config: old::InitSysConfig = extract_toml(&data_dir, "init_sys_config.toml")?;
        let genesis_block: old::Genesis = extract_toml(&data_dir, "genesis.toml")?;

        let key_id = extract_text(&data_dir, "key_id")?.parse()?;
        let kms_password = extract_text(&data_dir, "key_file")?;

        let this = Self {
            controller_port,
            consensus_port,
            executor_port,
            network_port,
            kms_port,
            storage_port,

            // controller
            node_addr,
            genesis_block,
            system_config,

            // kms
            kms_password,
            key_id,

            // network
            network_config,
        };

        Ok(this)
    }

    fn generate_new(&self) -> new::Config {
        let genesis_block = new::GenesisBlock {
            prevhash: self.genesis_block.prevhash.clone(),
            timestamp: self.genesis_block.timestamp,
        };

        let system_config = new::SystemConfig {
            admin: self.system_config.admin.clone(),
            block_interval: self.system_config.block_interval,
            block_limit: new::DEFAULT_BLOCK_LIMIT,
            chain_id: self.system_config.chain_id.clone(),
            validators: self.system_config.validators.clone(),
            version: self.system_config.version,
        };

        let controller = new::ControllerConfig {
            consensus_port: self.consensus_port,
            controller_port: self.controller_port,
            executor_port: self.executor_port,
            network_port: self.network_port,
            kms_port: self.kms_port,
            storage_port: self.storage_port,

            key_id: self.key_id,
            node_address: self.node_addr.clone(),
            package_limit: new::DEFAULT_PACKAGE_LIMIT,
        };

        let consensus = new::ConsensusRaftConfig {
            controller_port: self.controller_port,
            network_port: self.network_port,
            node_addr: self.node_addr.clone(),
            grpc_listen_port: self.consensus_port,
        };

        let kms = new::KmsSmConfig {
            kms_port: self.kms_port,
            db_key: self.kms_password.clone(),
        };

        let storage = new::StorageRocksDbConfig {
            kms_port: self.kms_port,
            storage_port: self.storage_port,
        };

        let executor = new::ExecutorEvmConfig {
            executor_port: self.executor_port,
        };

        let network = {
            let peers = self
                .network_config
                .peers
                .iter()
                .map(|p| {
                    new::NetworkTlsPeerConfig {
                        // will be filled latter
                        domain: None,
                        host: p.ip.clone(),
                        port: p.port,
                    }
                })
                .collect();

            new::NetworkTlsConfig {
                // will be filled latter
                ca_cert: None,
                cert: None,
                grpc_port: self.network_port,
                // listen network peers' connections
                listen_port: self.network_config.port,
                peers,
            }
        };

        new::Config {
            system_config,
            genesis_block,

            controller,
            consensus,
            executor,
            storage,
            kms,
            network,

            network_host: None,
            network_port: None,
        }
    }
}

fn extract_toml<T: DeserializeOwned>(data_dir: impl AsRef<Path>, file_name: &str) -> Result<T> {
    let s = extract_text(data_dir, file_name).context("cannot load toml file")?;
    let res: T = toml::from_str(&s)
        .with_context(|| format!("invalid toml for the `{}` type", std::any::type_name::<T>()))?;
    Ok(res)
}

fn extract_text(data_dir: impl AsRef<Path>, file_name: &str) -> Result<String> {
    let path = data_dir.as_ref().join(file_name);
    let mut f = File::open(&path).with_context(|| {
        format!(
            "cannot open file `{}` in `{}`",
            file_name,
            data_dir.as_ref().to_string_lossy()
        )
    })?;
    let mut buf = String::new();
    f.read_to_string(&mut buf)
        .with_context(|| format!("cannot read data from {}", path.to_string_lossy()))?;
    Ok(buf)
}

// Return CA's cert and key
fn fill_network_tls_info(node_configs: &mut [new::Config]) -> Result<CertAndKey> {
    // Construct (host, port) -> node_addr map.
    let host_port_to_addr: HashMap<(String, u16), String> = {
        let full_peer_set = {
            let mut full_peer_set = HashSet::<(String, u16)>::new();
            // Every node contains host and port for peers execept itself.
            // So we can construct the full set with two configs.
            for c in node_configs.iter().take(2) {
                for p in &c.network.peers {
                    full_peer_set.insert((p.host.clone(), p.port));
                }
            }
            full_peer_set
        };

        // Find nodes' host and port
        node_configs
            .iter_mut()
            .map(|c| {
                let peer_set: HashSet<(String, u16)> = c
                    .network
                    .peers
                    .iter()
                    .map(|p| (p.host.clone(), p.port))
                    .collect();
                let (host, port) = full_peer_set.difference(&peer_set).next().cloned()
                    .context(
                        "Cannot find out node's self host and port. \
                        The assumption that node's peers info contains all (and only) other peers has been violated"
                    )?;
                c.network_host.replace(host.clone());
                c.network_port.replace(port);

                Ok(
                    ((host, port), c.controller.node_address.clone())
                )
            })
            .collect::<Result<_>>()?
    };

    let node_addrs: Vec<String> = node_configs
        .iter()
        .map(|c| c.controller.node_address.clone())
        .collect();
    let (ca_cert_and_key, peer_cert_and_keys) = generate_certs(&node_addrs);

    node_configs
        .iter_mut()
        .zip(peer_cert_and_keys)
        .try_for_each(|(c, cert_and_key)| {
            c.network.ca_cert.replace(ca_cert_and_key.cert.clone());
            c.network.cert.replace(cert_and_key.cert);

            for p in c.network.peers.iter_mut() {
                let node_addr = host_port_to_addr
                    .get(&(p.host.clone(), p.port))
                    .cloned()
                    .with_context(|| {
                        format!(
                            "cannot find node address for `{}:{}`. go check network config",
                            &p.host, p.port
                        )
                    })?;
                p.domain.replace(node_addr);
            }
            Ok::<(), anyhow::Error>(())
        })?;

    Ok(ca_cert_and_key)
}

pub fn migrate<P, Q>(chain_data_dir: P, new_chain_data_dir: Q, chain_name: &str) -> Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let chain_data_dir = chain_data_dir.as_ref();
    let chain_metadata_dir = chain_data_dir.join(chain_name);
    ensure!(chain_data_dir.is_dir(), "chain data folder not found");
    ensure!(chain_metadata_dir.is_dir(), "metadata folder not found");

    let new_chain_data_dir = new_chain_data_dir.as_ref();
    let new_chain_metadata_dir = new_chain_data_dir.join(chain_name);
    fs::create_dir_all(&new_chain_data_dir).unwrap();
    fs::create_dir_all(&new_chain_metadata_dir).unwrap();

    // Load node dirs.
    let mut node_dirs: Vec<PathBuf> = fs::read_dir(chain_data_dir)
        .unwrap()
        .filter_map(|ent| {
            let ent = ent.unwrap();
            let dir_name = ent.file_name().into_string().unwrap();
            let prefix = format!("{}-", chain_name);
            if ent.file_type().unwrap().is_dir() && dir_name.starts_with(&prefix) {
                Some(ent.path())
            } else {
                None
            }
        })
        .collect();

    // Sort node dirs according to their node_id.
    node_dirs.sort_by_key(|d| {
        let dir_name = d.file_name().unwrap().to_string_lossy();
        let node_id: u64 = dir_name
            .strip_prefix(&format!("{}-", chain_name))
            .unwrap()
            .parse()
            .unwrap();
        node_id
    });

    // Construct new node config from the old one. (without network_tls info)
    let mut node_configs = node_dirs
        .iter()
        .map(|d| {
            NodeConfigMigrate::from_old(d)
                .with_context(|| format!("cannot migrate node config in `{}`", d.to_string_lossy()))
        })
        .collect::<Result<Vec<new::Config>>>()?;

    // Fill the network_tls info.
    let CertAndKey {
        cert: ca_cert_pem,
        key: ca_key_pem,
    } = fill_network_tls_info(&mut node_configs)
        .context("cannot fill network_tls info for chain config")?;

    // Construct $NEW_CHAIN_DATA_DIR/$CHAIN_NAME/config.toml
    let meta_config = {
        let node_addrs: Vec<String> = node_configs
            .iter()
            .map(|c| c.controller.node_address.clone())
            .collect();
        // Sample node
        let first_node = node_configs
            .first()
            .context("Empty chain. No node config found")?;
        let system_config = first_node.system_config.clone();
        let genesis_block = first_node.genesis_block.clone();

        let network_config = {
            let itself = new::NetworkTlsPeerConfig {
                domain: Some(first_node.controller.node_address.clone()),
                // Network info has been filled.
                host: first_node.network_host.clone().unwrap(),
                port: first_node.network_port.unwrap(),
            };
            let peers: Vec<new::NetworkTlsPeerConfig> = std::iter::once(itself)
                .chain(first_node.network.peers.clone())
                .collect();

            new::MetaNetworkConfig { peers }
        };

        let current_config = {
            let (ips, p2p_ports) = network_config
                .peers
                .iter()
                .map(|p| (p.host.clone(), p.port))
                .unzip();

            let rpc_ports = node_configs
                .iter()
                .map(|c| c.controller.controller_port)
                .collect();

            new::MetaCurrentConfig {
                addresses: node_addrs,
                ca_cert_pem,
                ca_key_pem,
                count: node_configs.len() as u64,

                ips,
                p2p_ports,
                rpc_ports,

                use_num: false,
                tls_peers: network_config.clone(),
            }
        };

        let admin_config = {
            let admin_address = first_node.system_config.admin.clone();
            let key_id = {
                let admin_key_dir = chain_metadata_dir.join(&admin_address);
                extract_text(admin_key_dir, "key_id")
                    .context("cannot load admin `key_id`")?
                    .parse()
                    .context("invalid admin `key_id`")?
            };
            new::MetaAdminConfig {
                admin_address,
                key_id,
            }
        };

        new::MetaConfig {
            network: network_config,
            genesis_block,
            system_config,
            admin_config,
            current_config,
        }
    };

    // construct new meta data
    let mut meta_config_toml = File::create(new_chain_metadata_dir.join("config.toml"))
        .context("cannot create meta `config.toml`")?;
    let meta_config_content = toml::to_string_pretty(&meta_config).unwrap();
    meta_config_toml
        .write_all(meta_config_content.as_bytes())
        .context("cannot write meta `config.toml`")?;

    let sample_node = node_dirs.first().unwrap();
    migrate_log4rs_and_kms_db(sample_node, new_chain_metadata_dir)
        .context("cannot copy log4rs and kms_db config to meta config dir")?;

    // construct new node data
    for (old_node_dir, node_config) in node_dirs.iter().zip(node_configs) {
        let new_node_dir = new_chain_data_dir.join(format!(
            "{}-{}",
            chain_name,
            node_config
                .controller
                .node_address
                .strip_prefix("0x")
                .context("invalid node address, must be a hex string with `0x` prefix")?
        ));
        fs::create_dir_all(&new_node_dir).with_context(|| {
            format!(
                "cannot create new node dir `{}`",
                new_node_dir.to_string_lossy()
            )
        })?;

        let mut node_config_toml = File::create(new_node_dir.join("config.toml"))
            .context("cannot create node's `config.toml`")?;
        let node_config_content = toml::to_string_pretty(&node_config).unwrap();
        node_config_toml
            .write_all(node_config_content.as_bytes())
            .context("cannot write node's `config.toml`")?;

        migrate_log4rs_and_kms_db(&old_node_dir, &new_node_dir).with_context(|| {
            format!(
                "cannot migrate log4rs yamls and kms db for `{}`",
                old_node_dir.to_string_lossy()
            )
        })?;
        migrate_chain_data_and_storage_data_and_logs(&old_node_dir, &new_node_dir).with_context(
            || {
                format!(
                    "cannot migrate {{chain data, storage data, logs}} for `{}`",
                    old_node_dir.to_string_lossy()
                )
            },
        )?;
    }

    Ok(())
}

fn migrate_log4rs_and_kms_db<P, Q>(old_dir: P, new_dir: Q) -> Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let old_dir = old_dir.as_ref();
    let new_dir = new_dir.as_ref();

    let files = [
        "controller-log4rs.yaml",
        "storage-log4rs.yaml",
        "executor-log4rs.yaml",
        "kms-log4rs.yaml",
        "kms.db",
    ];

    for f in files {
        let from = old_dir.join(f);
        let to = new_dir.join(f);
        fs::copy(&from, &to).with_context(|| {
            format!(
                "cannot copy file from `{}` to `{}`",
                from.to_string_lossy(),
                to.to_string_lossy()
            )
        })?;
    }
    Ok(())
}

fn migrate_chain_data_and_storage_data_and_logs<P, Q>(old_dir: P, new_dir: Q) -> Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let old_dir = old_dir.as_ref();
    let new_dir = new_dir.as_ref();

    let dirs = ["chain_data", "data", "logs"];

    let opts = CopyOptions {
        skip_exist: true,
        copy_inside: true,
        ..Default::default()
    };
    for d in dirs {
        let from = old_dir.join(d);
        let to = new_dir.join(d);
        copy_dir(&from, &to, &opts).with_context(|| {
            format!(
                "cannot copy dir from `{}` to `{}`",
                from.to_string_lossy(),
                to.to_string_lossy()
            )
        })?;
    }
    Ok(())
}
