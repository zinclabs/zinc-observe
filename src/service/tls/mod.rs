use std::{io::BufReader, sync::Arc};

use actix_tls::connect::rustls_0_23::{native_roots_cert_store, webpki_roots_cert_store};
use itertools::Itertools as _;
use rustls::{ClientConfig, ServerConfig};
use rustls_pemfile::{certs, private_key};
pub fn http_tls_config() -> Result<ServerConfig, anyhow::Error> {
    let cfg = config::get_config();
    let cert_file =
        &mut BufReader::new(std::fs::File::open(&cfg.http.tls_cert_path).map_err(|e| {
            anyhow::anyhow!(
                "Failed to open TLS certificate file {}: {}",
                &cfg.http.tls_cert_path,
                e
            )
        })?);
    let key_file =
        &mut BufReader::new(std::fs::File::open(&cfg.http.tls_key_path).map_err(|e| {
            anyhow::anyhow!(
                "Failed to open TLS key file {}: {}",
                &cfg.http.tls_key_path,
                e
            )
        })?);

    let cert_chain = certs(cert_file);
    // let mut keys = rsa_private_keys(key_file);
    let versions: &[&'_ rustls::SupportedProtocolVersion] = match cfg.http.tls_min_version.as_str()
    {
        "1.3" => &[&rustls::version::TLS13],
        "1.2" => rustls::DEFAULT_VERSIONS,
        _ => rustls::DEFAULT_VERSIONS,
    };

    let tls_config = ServerConfig::builder_with_protocol_versions(versions)
        .with_no_client_auth()
        .with_single_cert(
            cert_chain.try_collect::<_, Vec<_>, _>()?,
            private_key(key_file)?.unwrap(),
        )?;

    Ok(tls_config)
}

pub fn aws_client_tls_config() -> Result<Arc<ClientConfig>, anyhow::Error> {
    let cfg = config::get_config();
    let cert_store = match cfg.http.tls_root_certificates.as_str() {
        "webpki" => webpki_roots_cert_store(),
        "native" => native_roots_cert_store()?,
        _ => webpki_roots_cert_store(),
    };

    let mut config = ClientConfig::builder()
        .with_root_certificates(cert_store)
        .with_no_client_auth();

    let protos = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    config.alpn_protocols = protos;

    Ok(Arc::new(config))
}
