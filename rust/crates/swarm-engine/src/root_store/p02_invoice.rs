//! Opt-in invoice preparation through the in-process P02 vault.
use super::{P02Context, P02Grant, P02ResolvedValue, P02Vault, RootStore, RootStoreError as R};
use serde::Serialize;
use std::collections::BTreeSet;

/// One invoice supplied by the trusted local host. The identifier is approved
/// public metadata for this projection. The account reference is sensitive.
/// The input has no diagnostic or serialization implementation.
pub struct P02InvoiceInput<'a> {
    pub invoice_id: &'a str,
    pub account_reference: &'a [u8],
}

/// Closed agent-facing invoice. Token equality within the batch exposes matches.
#[derive(Debug, Serialize)]
pub struct P02AgentInvoice {
    invoice_id: String,
    account_token: String,
}

/// Closed reconciliation draft for the audience in the batch's mandate.
#[derive(Debug, Serialize)]
pub struct P02InvoiceDraft {
    invoice_id: String,
    account_matches: bool,
}

/// One prepared batch. Raw values live in its private vault. Keeping or dropping
/// this handle does not start an agent, publish a draft or settle an invoice.
pub struct P02InvoiceBatch<'root> {
    vault: P02Vault<'root>,
    invoices: Vec<P02AgentInvoice>,
}

impl RootStore {
    /// Validate nonempty, distinct invoice identifiers and tokenize each account
    /// under the issuer's mandate, using a fresh per-batch vault. The trusted host
    /// supplies disclosure-approved identifiers and bounds the input slice. If
    /// any row or authorization fails, the partially built vault is dropped and
    /// no projection is returned. No business state is written or sent.
    pub fn prepare_p02_invoice_batch(
        &self,
        grant: P02Grant,
        invoices: &[P02InvoiceInput<'_>],
        credential: &str,
    ) -> Result<P02InvoiceBatch<'_>, R> {
        let mut identifiers = BTreeSet::new();
        if invoices.is_empty()
            || invoices.iter().any(|invoice| {
                !swarm_identity_contract::name(invoice.invoice_id.as_bytes())
                    || !identifiers.insert(invoice.invoice_id)
            })
        {
            return Err(R::Refused("p02-invoice-identifiers"));
        }
        let context = grant.context.clone();
        let mut vault = self.open_p02_vault(grant, credential)?;
        let mut projected = Vec::with_capacity(invoices.len());
        for invoice in invoices {
            projected.push(P02AgentInvoice {
                invoice_id: invoice.invoice_id.to_owned(),
                account_token: vault.tokenize(&context, invoice.account_reference, credential)?,
            });
        }
        Ok(P02InvoiceBatch {
            vault,
            invoices: projected,
        })
    }
}

impl P02InvoiceBatch<'_> {
    /// Return only the closed projection to the issuer under current authority.
    /// A stale, foreign or revoked grant or alias refuses the whole projection.
    /// Previously delivered projections remain the caller's responsibility.
    pub fn project_for_agent(
        &self,
        context: &P02Context,
        credential: &str,
    ) -> Result<&[P02AgentInvoice], R> {
        self.vault.authorize_projection(
            context,
            credential,
            self.invoices
                .iter()
                .map(|entry| entry.account_token.as_str()),
        )?;
        Ok(&self.invoices)
    }

    /// Prepare a draft for two distinct invoices in this batch. The caller
    /// supplies the expected context and authenticated resolver credential.
    /// The boolean is computed under Trust from two active aliases. The draft
    /// contains only the subject invoice ID and the comparison result. No raw
    /// value is created, and the host retains the decision and means to send it.
    pub fn prepare_draft(
        &self,
        context: &P02Context,
        invoice_id: &str,
        reference_invoice_id: &str,
        credential: &str,
    ) -> Result<P02InvoiceDraft, R> {
        if invoice_id == reference_invoice_id {
            return Err(R::Refused("p02-invoice-comparison"));
        }
        let invoice = self.invoice(invoice_id)?;
        let reference = self.invoice(reference_invoice_id)?;
        let account_matches = self.vault.compare_aliases(
            context,
            &invoice.account_token,
            &reference.account_token,
            credential,
        )?;
        Ok(P02InvoiceDraft {
            invoice_id: invoice.invoice_id.clone(),
            account_matches,
        })
    }

    /// Resolve one known invoice's sensitive account for the local authorized
    /// resolver, separately from projection and draft. The vault checks current
    /// scope, mandate and alias. The caller owns the protected returned buffer.
    pub fn resolve_account(
        &self,
        context: &P02Context,
        invoice_id: &str,
        credential: &str,
    ) -> Result<P02ResolvedValue, R> {
        let invoice = self.invoice(invoice_id)?;
        self.vault
            .resolve(context, &invoice.account_token, credential)
    }

    /// Withdraw the account alias as the original issuer. All invoice rows that
    /// share the alias lose future resolution and draft authority. Returns false
    /// if already withdrawn. Prior outputs are outside the vault's custody.
    pub fn revoke_account(&mut self, invoice_id: &str, credential: &str) -> Result<bool, R> {
        let token = self.invoice(invoice_id)?.account_token.clone();
        self.vault.revoke(&token, credential)
    }

    /// Disable the batch mandate and drop all raw buffers under issuer authority.
    /// The vault refuses future projection, comparison and resolution.
    pub fn revoke_mandate(&mut self, credential: &str) -> Result<(), R> {
        self.vault.revoke_mandate(credential)
    }

    /// Read a known row from the immutable projection. Unknown IDs fail with
    /// a static error. The in-process host is responsible for guarding the batch.
    fn invoice(&self, id: &str) -> Result<&P02AgentInvoice, R> {
        self.invoices
            .iter()
            .find(|row| row.invoice_id == id)
            .ok_or(R::Refused("p02-invoice-unknown"))
    }
}
