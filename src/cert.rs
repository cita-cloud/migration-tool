use rcgen::BasicConstraints;
use rcgen::Certificate;
use rcgen::CertificateParams;
use rcgen::IsCa;
use rcgen::KeyPair;
use rcgen::PKCS_ECDSA_P256_SHA256;


pub struct CertAndKey {
    pub cert: String,
    pub key: String,
}


fn ca_cert() -> (Certificate, CertAndKey) {
    let mut params = CertificateParams::new(vec![]);
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);

    let keypair = KeyPair::generate(&PKCS_ECDSA_P256_SHA256).unwrap();
    params.key_pair.replace(keypair);

    let cert = Certificate::from_params(params).unwrap();
    let cert_and_key = {
        let cert_pem = cert.serialize_pem_with_signer(&cert).unwrap();
        let key_pem = cert.serialize_private_key_pem();
        CertAndKey {
            cert: cert_pem,
            key: key_pem
        }
    };

    (cert, cert_and_key)
}

fn cert(domain: &str, signer: &Certificate) -> (Certificate, CertAndKey) {
    let subject_alt_names = vec![domain.into()];
    let mut params = CertificateParams::new(subject_alt_names);

    let keypair = KeyPair::generate(&PKCS_ECDSA_P256_SHA256).unwrap();
    params.key_pair.replace(keypair);

    let cert = Certificate::from_params(params).unwrap();
    let cert_pem = cert.serialize_pem_with_signer(signer).unwrap();
    let cert_and_key = {
        let cert_pem = cert.serialize_pem_with_signer(signer).unwrap();
        let key_pem = cert.serialize_private_key_pem();
        CertAndKey {
            cert: cert_pem,
            key: key_pem
        }
    };
    (cert, cert_and_key)
}


pub fn generate_certs(domains: &[String]) -> (CertAndKey, Vec<CertAndKey>) {
    let (ca_cert, ca_cert_and_key) = ca_cert();
    let peer_cert_and_keys = domains.iter().map(|domain| cert(domain, &ca_cert).1).collect();

    (ca_cert_and_key, peer_cert_and_keys)
}

