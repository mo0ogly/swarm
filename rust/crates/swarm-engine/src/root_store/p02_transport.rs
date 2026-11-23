//! A one-draft local transport over an already connected socket.
use super::{P02Context, P02InvoiceBatch, RootStoreError};
use sha2::{Digest, Sha256};
use std::{
    fmt,
    io::{self, Read, Write},
    net::Shutdown,
    os::unix::net::UnixStream,
};

/// One connected channel for one authorized comparison. The trusted host owns
/// peer selection and binds the same context and reference on the receiver side
/// before passing the stream. These fields are never transmitted. Keep the
/// stream private to those peers and set I/O deadlines before handing it over.
pub struct P02DraftDestination {
    pub stream: UnixStream,
    pub context: P02Context,
    pub reference_invoice_id: String,
}

/// Successful acknowledgement of the exact JSON bytes on this channel.
/// The digest correlates the response. It grants no authority or proof of
/// persistence, and is meaningful only for the peer selected by the host.
#[derive(Debug, PartialEq, Eq)]
pub struct P02DraftReceipt {
    pub sha256: [u8; 32],
}

/// Failure at the preparation or receipt boundary. Once socket I/O is attempted,
/// the receiver's effect may be unknown. The transport never retries.
#[derive(Debug)]
pub enum P02DraftDeliveryError {
    Preparation(RootStoreError),
    Encoding(serde_json::Error),
    Unconfirmed(io::Error),
    InvalidReceipt,
}

impl fmt::Display for P02DraftDeliveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Preparation(_) => f.write_str("P02 draft preparation failed before send"),
            Self::Encoding(_) => f.write_str("P02 draft encoding failed before send"),
            Self::Unconfirmed(_) => f.write_str("P02 draft delivery is unconfirmed"),
            Self::InvalidReceipt => f.write_str("P02 draft acknowledgement is invalid"),
        }
    }
}

impl std::error::Error for P02DraftDeliveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Preparation(error) => Some(error),
            Self::Encoding(error) => Some(error),
            Self::Unconfirmed(error) => Some(error),
            Self::InvalidReceipt => None,
        }
    }
}

impl P02InvoiceBatch<'_> {
    /// Compare the subject and destination-bound reference under current Trust,
    /// then send the closed draft over the consumed socket. A preparation error
    /// writes no bytes. The write half closes after the JSON, and the peer must
    /// reply with its 32-byte SHA-256 digest followed by EOF. Empty, truncated,
    /// oversized or mismatched receipts fail. A missing confirmation may hide
    /// receipt of the draft. Revocation prevents future preparations; this call
    /// holds no Trust reservation during I/O and cannot recall authorized bytes.
    pub fn deliver_draft(
        &self,
        destination: P02DraftDestination,
        invoice_id: &str,
        credential: &str,
    ) -> Result<P02DraftReceipt, P02DraftDeliveryError> {
        let P02DraftDestination {
            mut stream,
            context,
            reference_invoice_id,
        } = destination;
        let draft = self
            .prepare_draft(&context, invoice_id, &reference_invoice_id, credential)
            .map_err(P02DraftDeliveryError::Preparation)?;
        let bytes = serde_json::to_vec(&draft).map_err(P02DraftDeliveryError::Encoding)?;
        let sha256: [u8; 32] = Sha256::digest(&bytes).into();
        stream
            .write_all(&bytes)
            .and_then(|()| stream.shutdown(Shutdown::Write))
            .map_err(P02DraftDeliveryError::Unconfirmed)?;

        // Observe one byte past the exact receipt width to reject trailing data.
        let mut reply = Vec::with_capacity(sha256.len() + 1);
        stream
            .take((sha256.len() + 1) as u64)
            .read_to_end(&mut reply)
            .map_err(P02DraftDeliveryError::Unconfirmed)?;
        if reply != sha256 {
            return Err(P02DraftDeliveryError::InvalidReceipt);
        }
        Ok(P02DraftReceipt { sha256 })
    }
}
