//! Provides a couple of functions that assist in getting a `TlsAcceptor` from certificate and key data.
//!
//! These functions use safe defaults from rustls to generate the `TlsAcceptor`, but it is not necessary to use them.

use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Cursor};
use std::path::Path;
use std::sync::Arc;
use tokio_rustls::rustls;

/// The HTTP protocol to use when clients are connecting.
///
/// This should match the version(s) of HTTP used to serve your application in Hyper.
/// Using `Both` will prefer HTTP/2 over HTTP/1.1
pub enum HttpProtocol {
    Http1,
    Http2,
    Both,
}

/// Get a `TlsAcceptor` from PEM certificate and key data
///
/// # Errors
/// Errors if there is no valid certificate/key data given, or if rustls fails to create
/// the server config
pub fn get_tlsacceptor_from_pem_data(
    cert_data: &str,
    key_data: &str,
    protocol: &HttpProtocol,
) -> Result<tokio_rustls::TlsAcceptor, Box<dyn Error>> {
    let mut cert_reader = BufReader::new(Cursor::new(cert_data));
    let mut key_reader = BufReader::new(Cursor::new(key_data));
    get_tlsacceptor_from_readers(&mut cert_reader, &mut key_reader, protocol)
}

/// Get a `TlsAcceptor` from PEM-encoded certificate and key files
///
/// # Errors
/// Errors if the files cannot be read, if there is no valid certificate/key data given, or if rustls fails to create
/// the server config
pub fn get_tlsacceptor_from_files(
    cert_path: impl AsRef<Path>,
    key_path: impl AsRef<Path>,
    protocol: &HttpProtocol,
) -> Result<tokio_rustls::TlsAcceptor, Box<dyn Error>> {
    let cert_file = File::open(cert_path)?;
    let key_file = File::open(key_path)?;

    let mut cert_reader = BufReader::new(cert_file);
    let mut key_reader = BufReader::new(key_file);

    get_tlsacceptor_from_readers(&mut cert_reader, &mut key_reader, protocol)
}

fn get_tlsacceptor_from_readers(
    cert_reader: &mut dyn BufRead,
    key_reader: &mut dyn BufRead,
    protocol: &HttpProtocol,
) -> Result<tokio_rustls::TlsAcceptor, Box<dyn Error>> {
    let certs: Vec<_> = rustls_pemfile::certs(cert_reader)?
        .into_iter()
        .map(rustls::Certificate)
        .collect();

    let key = rustls_pemfile::read_one(key_reader)?.ok_or("no valid pem data in key data")?;
    let key = match key {
        rustls_pemfile::Item::ECKey(data)
        | rustls_pemfile::Item::RSAKey(data)
        | rustls_pemfile::Item::PKCS8Key(data) => rustls::PrivateKey(data),
        _ => return Err("no private key in key data".into()),
    };

    let mut cfg = rustls::server::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    cfg.alpn_protocols = match protocol {
        HttpProtocol::Http1 => vec![b"http/1.1".to_vec(), b"http/1.0".to_vec()],
        HttpProtocol::Http2 => vec![b"h2".to_vec()],
        HttpProtocol::Both => vec![b"h2".to_vec(), b"http/1.1".to_vec(), b"http/1.0".to_vec()],
    };

    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(cfg));

    Ok(acceptor)
}