use indexer_core::{
    db::{tables::metadatas, update},
    prelude::*,
};

use super::Client;
use crate::prelude::*;

pub(crate) async fn process_burn_instruction(
    client: &Client,
    accounts: &[Pubkey],
    slot: u64,
) -> Result<()> {
    if accounts.len() != 3 {
        return Ok(());
    }

    let mint = accounts[2].to_string();
    let slot = i64::try_from(slot)?;

    client
        .db()
        .run(move |db| {
            update(metadatas::table.filter(metadatas::mint_address.eq(mint)))
                .set((metadatas::burned.eq(true), metadatas::slot.eq(slot)))
                .execute(db)
        })
        .await
        .context("failed to update metadata")?;

    Ok(())
}
