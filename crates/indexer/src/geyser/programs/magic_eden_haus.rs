use borsh::BorshDeserialize;
use indexer_core::{
    db::{
        models::{Listing, Offer, Purchase},
        tables::{listings, offers, purchases},
        update,
    },
    pubkeys, util,
    uuid::Uuid,
};

use super::{
    instructions::{
        buy::upsert_into_offers_table, execute_sale::upsert_into_purchases_table,
        sell::upsert_into_listings_table,
    },
    Client,
};
use crate::prelude::*;

const BUY: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
const SELL: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];
const EXECUTE_SALE: [u8; 8] = [37, 74, 217, 157, 79, 49, 35, 6];
const CANCEL_SELL: [u8; 8] = [198, 198, 130, 203, 163, 95, 175, 75];
const CANCEL_BUY: [u8; 8] = [238, 76, 36, 218, 132, 177, 224, 233];

#[derive(BorshDeserialize, Debug, Clone)]
struct MEInstructionData {
    trade_state_bump: u8,
    _escrow_payment_bump: u8,
    buyer_price: u64,
    token_size: u64,
    expiry: i64,
}

async fn process_execute_sale(
    client: &Client,
    mut data: &[u8],
    accounts: &[Pubkey],
    slot: u64,
) -> Result<()> {
    let params = MEInstructionData::deserialize(&mut data)
        .context("failed to deserialize ME ExecuteSale instruction")?;

    let accts: Vec<_> = accounts.iter().map(ToString::to_string).collect();

    upsert_into_purchases_table(
        client,
        Purchase {
            id: None,
            buyer: Owned(accts[0].clone()),
            seller: Owned(accts[1].clone()),
            auction_house: Owned(accts[9].clone()),
            marketplace_program: Owned(pubkeys::ME_HAUS.to_string()),
            metadata: Owned(accts[5].clone()),
            token_size: params.token_size.try_into()?,
            price: params.buyer_price.try_into()?,
            created_at: Utc::now().naive_utc(),
            slot: slot.try_into()?,
            write_version: None,
        },
        accts[11].clone(),
        accts[13].clone(),
    )
    .await
    .context("failed to insert listing!")?;

    Ok(())
}

async fn process_sale(
    client: &Client,
    mut data: &[u8],
    accounts: &[Pubkey],
    slot: u64,
) -> Result<()> {
    let params = MEInstructionData::deserialize(&mut data)
        .context("failed to deserialize ME Sell instruction")?;

    let accts: Vec<_> = accounts.iter().map(ToString::to_string).collect();

    let seller = accts[0].clone();
    let auction_house = accts[7].clone();
    let metadata = accts[5].clone();
    let price = i64::try_from(params.buyer_price)?;
    let token_size = i64::try_from(params.token_size)?;
    let slot = i64::try_from(slot)?;

    let purchase_id = client
        .db()
        .run({
            move |db| {
                purchases::table
                    .filter(
                        purchases::seller
                            .eq(seller)
                            .and(purchases::auction_house.eq(auction_house))
                            .and(purchases::metadata.eq(metadata))
                            .and(purchases::price.eq(price))
                            .and(
                                purchases::token_size
                                    .eq(token_size)
                                    .and(purchases::slot.eq(slot)),
                            ),
                    )
                    .select(purchases::id)
                    .first::<Uuid>(db)
                    .optional()
                    .context("failed to get purchase ids")
            }
        })
        .await?;

    upsert_into_listings_table(client, Listing {
        id: None,
        trade_state: Owned(accts[8].clone()),
        auction_house: Owned(accts[7].clone()),
        marketplace_program: Owned(pubkeys::ME_HAUS.to_string()),
        seller: Owned(accts[0].clone()),
        metadata: Owned(accts[5].clone()),
        purchase_id,
        price: params.buyer_price.try_into()?,
        token_size: params.token_size.try_into()?,
        trade_state_bump: params.trade_state_bump.try_into()?,
        created_at: Utc::now().naive_utc(),
        canceled_at: None,
        slot,
        write_version: None,
        expiry: match params.expiry {
            e if e <= 0 => None,
            _ => Some(util::unix_timestamp(params.expiry)?),
        },
    })
    .await
    .context("failed to insert listing!")?;

    Ok(())
}

async fn process_buy(
    client: &Client,
    mut data: &[u8],
    accounts: &[Pubkey],
    slot: u64,
) -> Result<()> {
    let params = MEInstructionData::deserialize(&mut data)
        .context("failed to deserialize ME Buy instruction")?;

    if accounts.len() != 12 {
        debug!("invalid accounts for BuyInstruction");
        return Ok(());
    }

    let accts: Vec<_> = accounts.iter().map(ToString::to_string).collect();
    let buyer = accts[0].clone();
    let auction_house = accts[6].clone();
    let metadata = accts[3].clone();
    let price = i64::try_from(params.buyer_price)?;
    let token_size = i64::try_from(params.token_size)?;
    let slot = i64::try_from(slot)?;

    let purchase_id: Option<Uuid> = client
        .db()
        .run({
            move |db| {
                purchases::table
                    .filter(
                        purchases::buyer
                            .eq(buyer)
                            .and(purchases::auction_house.eq(auction_house))
                            .and(purchases::metadata.eq(metadata))
                            .and(purchases::price.eq(price))
                            .and(
                                purchases::token_size
                                    .eq(token_size)
                                    .and(purchases::slot.eq(slot)),
                            ),
                    )
                    .select(purchases::id)
                    .first::<Uuid>(db)
                    .optional()
                    .context("failed to get purchase ids")
            }
        })
        .await?;

    let offer = Offer {
        id: None,
        trade_state: Owned(accts[7].clone()),
        auction_house: Owned(accts[6].clone()),
        marketplace_program: Owned(pubkeys::ME_HAUS.to_string()),
        buyer: Owned(accts[0].clone()),
        metadata: Owned(accts[3].clone()),
        token_account: None,
        purchase_id,
        price,
        token_size: params.token_size.try_into()?,
        trade_state_bump: params.trade_state_bump.try_into()?,
        created_at: Utc::now().naive_utc(),
        canceled_at: None,
        slot,
        write_version: None,
        expiry: match params.expiry {
            e if e <= 0 => None,
            _ => Some(util::unix_timestamp(params.expiry)?),
        },
    };

    upsert_into_offers_table(client, offer)
        .await
        .context("failed to insert offer")?;

    Ok(())
}

async fn process_cancel_sale(client: &Client, accounts: &[Pubkey], slot: u64) -> Result<()> {
    let accts: Vec<_> = accounts.iter().map(ToString::to_string).collect();
    let canceled_at = Utc::now().naive_utc();
    let trade_state = accts[6].clone();
    let slot = i64::try_from(slot)?;

    client
        .db()
        .run(move |db| {
            update(
                listings::table.filter(
                    listings::trade_state
                        .eq(trade_state)
                        .and(listings::purchase_id.is_null())
                        .and(listings::canceled_at.is_null()),
                ),
            )
            .set((
                listings::canceled_at.eq(Some(canceled_at)),
                listings::slot.eq(slot),
            ))
            .execute(db)
        })
        .await
        .context("failed to cancel ME listing ")?;

    Ok(())
}

async fn process_cancel_buy(client: &Client, accounts: &[Pubkey], slot: u64) -> Result<()> {
    let accts: Vec<_> = accounts.iter().map(ToString::to_string).collect();
    let canceled_at = Utc::now().naive_utc();
    let trade_state = accts[5].clone();
    let slot = i64::try_from(slot)?;

    client
        .db()
        .run(move |db| {
            update(
                offers::table.filter(
                    offers::trade_state
                        .eq(trade_state)
                        .and(offers::purchase_id.is_null())
                        .and(offers::canceled_at.is_null()),
                ),
            )
            .set((
                offers::canceled_at.eq(Some(canceled_at)),
                offers::slot.eq(slot),
            ))
            .execute(db)
        })
        .await
        .context("failed to cancel ME bid ")?;

    Ok(())
}

pub(crate) async fn process_instruction(
    client: &Client,
    data: &[u8],
    accounts: &[Pubkey],
    slot: u64,
) -> Result<()> {
    let (discriminator, params) = data.split_at(8);
    let discriminator = <[u8; 8]>::try_from(discriminator)?;

    match discriminator {
        BUY => process_buy(client, params, accounts, slot).await,
        SELL => process_sale(client, params, accounts, slot).await,
        EXECUTE_SALE => process_execute_sale(client, params, accounts, slot).await,
        CANCEL_SELL => process_cancel_sale(client, accounts, slot).await,
        CANCEL_BUY => process_cancel_buy(client, accounts, slot).await,
        _ => Ok(()),
    }
}
