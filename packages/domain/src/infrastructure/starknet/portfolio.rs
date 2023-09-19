use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{
        models::{BlockId, BlockTag},
        HttpTransport, JsonRpcClient,
    },
};

use crate::{
    domain::{crypto::U256, Erc721, SlotValue},
    infrastructure::{
        postgres::entity::ErcImplementation,
        view_model::{
            customer::CustomerToken,
            portfolio::{Erc3525Token, ProjectWithMinterAndPaymentViewModel, Token},
        },
    },
};

use super::{
    get_starknet_rpc_from_env,
    model::{felt_to_u256, get_call_function, u256_to_felt, ModelError, StarknetModel},
    uri::UriModel,
};

pub(crate) async fn get_balance_of(
    provider: &JsonRpcClient<HttpTransport>,
    address: &str,
    wallet: &str,
) -> Result<u64, ModelError> {
    let response = provider
        .call(
            get_call_function(
                &FieldElement::from_hex_be(address).unwrap(),
                "balanceOf",
                vec![FieldElement::from_hex_be(wallet).unwrap()],
            ),
            &BlockId::Tag(BlockTag::Latest),
        )
        .await?;

    Ok(u64::try_from(response.first().unwrap().to_owned()).unwrap())
}

pub(crate) async fn get_token_id(
    provider: &JsonRpcClient<HttpTransport>,
    address: &str,
    wallet: &str,
    index: &u64,
) -> Result<U256, ModelError> {
    let response = provider
        .call(
            get_call_function(
                &FieldElement::from_hex_be(address).unwrap(),
                "tokenOfOwnerByIndex",
                vec![
                    FieldElement::from_hex_be(wallet).unwrap(),
                    FieldElement::from(*index),
                    FieldElement::ZERO,
                ],
            ),
            &BlockId::Tag(BlockTag::Latest),
        )
        .await?;

    Ok(felt_to_u256(response.first().unwrap().to_owned()))
}

async fn get_token_uri(
    provider: &JsonRpcClient<HttpTransport>,
    address: &str,
    token_id: &U256,
    implementation: ErcImplementation,
) -> Result<String, ModelError> {
    let entrypoint = match implementation {
        ErcImplementation::Erc721 => "tokenURI",
        ErcImplementation::Erc3525 => "token_uri",
        ErcImplementation::Enum => panic!("this should be a valid erc implementation"),
    };
    let response = provider
        .call(
            get_call_function(
                &FieldElement::from_hex_be(address).unwrap(),
                entrypoint,
                vec![u256_to_felt(token_id), FieldElement::ZERO],
            ),
            &BlockId::Tag(BlockTag::Latest),
        )
        .await?;
    let string: String = response
        .iter()
        .skip(1)
        .map(|fe| {
            fe.to_bytes_be()
                .to_vec()
                .iter()
                .filter(|b| 0 != **b)
                .copied()
                .collect()
        })
        .map(|bytes| unsafe { String::from_utf8_unchecked(bytes) })
        .collect();

    Ok(string)
}

pub(crate) async fn get_value_of_token_in_slot(
    provider: &JsonRpcClient<HttpTransport>,
    address: &str,
    token_id: &U256,
) -> Result<U256, ModelError> {
    let response = provider
        .call(
            get_call_function(
                &FieldElement::from_hex_be(address).unwrap(),
                "value_of",
                vec![u256_to_felt(token_id), FieldElement::ZERO],
            ),
            &BlockId::Tag(BlockTag::Latest),
        )
        .await?;
    Ok(felt_to_u256(response.first().unwrap().to_owned()))
}

/// Load ERC-721 portfolio from starknet data
pub async fn load_erc_721_portfolio(address: &str, wallet: &str) -> Result<Vec<Token>, ModelError> {
    let provider = get_starknet_rpc_from_env()?;
    let mut tokens = vec![];
    // balance
    let balance = get_balance_of(&provider, address, wallet).await?;
    for token_index in 0..balance {
        // tokenOfOwnerByIndex(owner, index) -> tokenId
        let token_id = get_token_id(&provider, address, wallet, &token_index).await?;
        // tokenURI
        let token_uri =
            get_token_uri(&provider, address, &token_id, ErcImplementation::Erc721).await?;
        let uri_model = UriModel::<Erc721>::new(token_uri)?;
        let metadata = uri_model.load().await?;
        tokens.push(Token {
            token_id,
            image: metadata.image,
            name: metadata.name,
        });
    }

    Ok(tokens)
}

/// Load ERC-3525 portfolio from starknet data
pub async fn load_erc_3525_portfolio(
    project: &ProjectWithMinterAndPaymentViewModel,
    address: &str,
    customer_tokens: &[CustomerToken],
) -> Result<Vec<Option<Erc3525Token>>, ModelError> {
    let provider = get_starknet_rpc_from_env()?;
    let mut tokens = vec![];
    for token_index in customer_tokens {
        let token_uri = get_token_uri(
            &provider,
            address,
            &token_index.token_id,
            ErcImplementation::Erc3525,
        )
        .await?;
        let value: U256 =
            get_value_of_token_in_slot(&provider, address, &token_index.token_id).await?;
        // let value: U256 = StarknetValue::new(data[1].clone()).resolve("u256").into();

        tokens.push(Some(Erc3525Token {
            token_id: token_index.token_id,
            name: project.name.to_string(),
            value,
            image: token_uri,
            slot_value: SlotValue::from_blockchain(value, project.value_decimals).into(),
        }));
    }

    Ok(tokens)
}
