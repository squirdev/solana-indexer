use objects::{
    creator::Creator,
    marketplace::Marketplace,
    nft::Nft,
    profile::{Profile, TwitterProfilePictureResponse, TwitterShowResponse},
    storefront::Storefront,
    wallet::Wallet,
};
use tables::{
    attributes, metadata_creators, metadata_jsons, metadatas, store_config_jsons, storefronts,
    token_accounts,
};

use super::prelude::*;

pub struct QueryRoot;

#[derive(GraphQLInputObject, Clone, Debug)]
#[graphql(description = "Filter on NFT attributes")]
struct AttributeFilter {
    trait_type: String,
    values: Vec<String>,
}

#[graphql_object(Context = AppContext)]
impl QueryRoot {
    async fn profile(
        &self,
        ctx: &AppContext,
        #[graphql(description = "Twitter handle")] handle: String,
    ) -> Option<Profile> {
        let twitter_bearer_token = &ctx.twitter_bearer_token;
        let http_client = reqwest::Client::new();

        let twitter_show_response: TwitterShowResponse = http_client
            .get("https://api.twitter.com/1.1/users/show.json")
            .header("Accept", "application/json")
            .query(&[("screen_name", &handle)])
            .bearer_auth(twitter_bearer_token)
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()?;

        let twitter_profile_picture_response: TwitterProfilePictureResponse = http_client
            .get(format!(
                "https://api.twitter.com/2/users/by/username/{}",
                handle
            ))
            .header("Accept", "application/json")
            .query(&[("user.fields", "profile_image_url")])
            .bearer_auth(twitter_bearer_token)
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()?;

        Some(Profile::from((
            twitter_profile_picture_response,
            twitter_show_response,
        )))
    }

    fn creator(
        &self,
        _context: &AppContext,
        #[graphql(description = "Address of creator")] address: String,
    ) -> Creator {
        Creator { address }
    }

    fn nfts(
        &self,
        context: &AppContext,
        #[graphql(description = "Filter on owner address")] owners: Option<Vec<String>>,
        #[graphql(description = "Filter on creator address")] creators: Option<Vec<String>>,
        #[graphql(description = "Filter on attributes")] attributes: Option<Vec<AttributeFilter>>,
    ) -> FieldResult<Vec<Nft>> {
        if owners.is_none() && creators.is_none() {
            return Err(FieldError::new(
                "No filter provided! Please provide at least one of the filters",
                graphql_value!({ "Filters": "owners: Vec<String>, creators: Vec<String>" }),
            ));
        }
        if owners.is_some() && creators.is_some() {
            return Err(FieldError::new(
                "Please pass either owners or creators, not both",
                graphql_value!({ "Filters": "owners: Vec<String>, creators: Vec<String>" }),
            ));
        }

        let conn = context.db_pool.get().context("failed to connect to db")?;

        let query = metadatas::table.into_boxed();
        let query = attributes.unwrap_or_else(Vec::new).into_iter().fold(
            query,
            |acc, AttributeFilter { trait_type, values }| {
                let sub = attributes::table
                    .select(attributes::metadata_address)
                    .filter(
                        attributes::trait_type
                            .eq(trait_type)
                            .and(attributes::value.eq(any(values))),
                    );

                acc.filter(metadatas::address.eq(any(sub)))
            },
        );

        let rows: Vec<models::Nft> = if let Some(creators) = creators {
            query
                .inner_join(
                    metadata_creators::table
                        .on(metadatas::address.eq(metadata_creators::metadata_address)),
                )
                .inner_join(
                    metadata_jsons::table
                        .on(metadatas::address.eq(metadata_jsons::metadata_address)),
                )
                .filter(metadata_creators::creator_address.eq(any(creators)))
                .select((
                    metadatas::address,
                    metadatas::name,
                    metadatas::seller_fee_basis_points,
                    metadatas::mint_address,
                    metadatas::primary_sale_happened,
                    metadata_jsons::description,
                    metadata_jsons::image,
                ))
                .order_by(metadatas::name.desc())
                .load(&conn)
                .context("failed to load nft(s)")?
        } else if let Some(owners) = owners {
            // owners
            query
                .inner_join(
                    token_accounts::table
                        .on(metadatas::mint_address.eq(token_accounts::mint_address)),
                )
                .inner_join(
                    metadata_jsons::table
                        .on(metadatas::address.eq(metadata_jsons::metadata_address)),
                )
                .filter(token_accounts::amount.eq(1))
                .filter(token_accounts::owner_address.eq(any(owners)))
                .select((
                    metadatas::address,
                    metadatas::name,
                    metadatas::seller_fee_basis_points,
                    metadatas::mint_address,
                    metadatas::primary_sale_happened,
                    metadata_jsons::description,
                    metadata_jsons::image,
                ))
                .order_by(metadatas::name.desc())
                .load(&conn)
                .context("failed to load nft(s)")?
        } else {
            unreachable!("something has gone horribly wrong on NFTs query");
        };

        Ok(rows.into_iter().map(Into::into).collect())
    }

    fn wallet(
        &self,
        _context: &AppContext,
        #[graphql(description = "Address of NFT")] address: String,
    ) -> Option<Wallet> {
        Some(Wallet { address })
    }

    fn nft(
        &self,
        context: &AppContext,
        #[graphql(description = "Address of NFT")] address: String,
    ) -> FieldResult<Option<Nft>> {
        let conn = context.db_pool.get()?;
        let mut rows: Vec<models::Nft> = metadatas::table
            .inner_join(
                metadata_jsons::table.on(metadatas::address.eq(metadata_jsons::metadata_address)),
            )
            .filter(metadatas::address.eq(address))
            .select((
                metadatas::address,
                metadatas::name,
                metadatas::seller_fee_basis_points,
                metadatas::mint_address,
                metadatas::primary_sale_happened,
                metadata_jsons::description,
                metadata_jsons::image,
            ))
            .limit(1)
            .load(&conn)
            .context("Failed to load metadata")?;

        Ok(rows.pop().map(Into::into))
    }

    #[graphql(description = "A storefront")]
    fn storefront(
        &self,
        context: &AppContext,
        subdomain: String,
    ) -> FieldResult<Option<Storefront>> {
        let columns = (
            storefronts::owner_address,
            storefronts::subdomain,
            storefronts::title,
            storefronts::description,
            storefronts::favicon_url,
            storefronts::logo_url,
            storefronts::updated_at,
            storefronts::banner_url,
            storefronts::address,
        );

        let conn = context.db_pool.get()?;
        let mut rows: Vec<models::Storefront> = storefronts::table
            .filter(storefronts::subdomain.eq(subdomain))
            .select(columns)
            .limit(1)
            .load(&conn)
            .context("Failed to load storefront")?;

        Ok(rows.pop().map(Into::into))
    }

    #[graphql(description = "A marketplace")]
    fn marketplace(
        &self,
        context: &AppContext,
        subdomain: String,
    ) -> FieldResult<Option<Marketplace>> {
        let conn = context.db_pool.get()?;
        let mut rows: Vec<models::StoreConfigJson> = store_config_jsons::table
            .filter(store_config_jsons::subdomain.eq(subdomain))
            .select(store_config_jsons::all_columns)
            .limit(1)
            .load(&conn)
            .context("Failed to load store config JSON")?;

        Ok(rows.pop().map(Into::into))
    }
}