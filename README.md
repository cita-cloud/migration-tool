# Migration Tool

A migration tool for upgrading CITA-Cloud chain from v6.1.0 to v6.3.0.


## Usage
**WARNING: Backup your data before use it**.


```
$ migration-tool help migrate

migration-tool-migrate

Migrate the chain data

USAGE:
    migration-tool migrate --chain-dir <chain-dir> --out-dir <out-dir> --chain-name <chain-name>

OPTIONS:
    -d, --chain-dir <chain-dir>      The old chain dir
    -h, --help                       Print help information
    -n, --chain-name <chain-name>    Name of the chain
    -o, --out-dir <out-dir>          The output dir for the upgraded chain
```


The expected chain data structure:
```
$ tree old-chain

old-chain
├── test-chain
│   └── 0xf6c4510431728d0b6a3ef64b49c08cbab5c49da6
│       ├── key_file
│       ├── key_id
│       ├── kms.db
│       └── node_address
├── test-chain-0
│   ├── chain_data
|   |       (omitted)
│   ├── config.toml
│   ├── consensus-config.toml
│   ├── consensus-log4rs.yaml
│   ├── controller-config.toml
│   ├── controller-log4rs.yaml
│   ├── data
|   |       (omitted)
│   ├── executor-log4rs.yaml
│   ├── genesis.toml
│   ├── init_sys_config.toml
│   ├── key_file
│   ├── key_id
│   ├── kms.db
│   ├── kms-log4rs.yaml
│   ├── logs
|   |       (omitted)
│   ├── network-config.toml
│   ├── network_key
│   ├── network-log4rs.yaml
│   ├── node_address
│   ├── node_key
│   ├── raft-data-dir
|   |       (omitted)
│   └── storage-log4rs.yaml
|── test-chain-1
|       (omitted)
|── test-chain-2
|       (omitted)
└── test-chain-3
        (omitted)
```

The output chain data structure:
```
$ tree new-chain

new-chain
├── test-chain
│   ├── config.toml
│   ├── controller-log4rs.yaml
│   ├── executor-log4rs.yaml
│   ├── kms.db
│   ├── kms-log4rs.yaml
│   └── storage-log4rs.yaml
├── test-chain-3f91e1969fc0a43d8a3429ce07e3a691533093a5
│   ├── chain_data
│   |       (omitted)
│   ├── config.toml
│   ├── controller-log4rs.yaml
│   ├── data
|   |       (omitted)
│   ├── executor-log4rs.yaml
│   ├── kms.db
│   ├── kms-log4rs.yaml
│   └── logs
|           (omitted)
├── test-chain-455379ad72e28341e0d9cfe0dd5cd6eec9d884ad
│       (omitted)
├── test-chain-554c40e97c3f81bb1cdbadd042f8e3ef1930ee9c
|       (omitted)
└── test-chain-f65bff66ab713523d7191c499d3fcf80231de9a8
        (omitted)
```

`kms.db`, `data`, `chain_data` and `logs` will be copied to the corresponding new node directory.

## Q & A
Q: How can I understand this migration process?

A: Go check the old and new chain config.

</br>

Q: Where is the `consensus-log4rs.yaml`?

A: This tool only upgrate to `consensus_raft` which doesn't use `log4rs`.

</br>

Q: Where is my `consensus_raft` data?

A: Discarded. The new `consensus_raft` has a incompatible wal data. This migration will reset the raft state and use controller's block hight. It should work fine.
