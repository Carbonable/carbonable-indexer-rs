use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{
        models::{BlockId, BlockTag},
        HttpTransport, JsonRpcClient,
    },
};

use crate::{
    domain::{crypto::U256, SlotValue},
    infrastructure::view_model::{
        customer::CustomerToken,
        portfolio::{Erc3525Token, ProjectWithMinterAndPaymentViewModel, Token},
    },
};

use super::{
    get_starknet_rpc_from_env,
    model::{felt_to_u256, get_call_function, u256_to_felt, ModelError},
};

pub(crate) async fn get_balance_of(
    provider: &JsonRpcClient<HttpTransport>,
    address: &str,
    wallet: &str,
) -> Result<u64, ModelError> {
    let response = match provider
        .call(
            get_call_function(
                &FieldElement::from_hex_be(address).unwrap(),
                "balanceOf",
                vec![FieldElement::from_hex_be(wallet).unwrap()],
            ),
            &BlockId::Tag(BlockTag::Pending),
        )
        .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("failed to get balanceOf({address},{wallet}) {:?}", e);
            return Err(ModelError::ProviderError(e));
        }
    };

    Ok(u64::try_from(response.first().unwrap().to_owned()).unwrap())
}

pub(crate) async fn get_token_id(
    provider: &JsonRpcClient<HttpTransport>,
    address: &str,
    wallet: &str,
    index: &u64,
) -> Result<U256, ModelError> {
    let response = match provider
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
            &BlockId::Tag(BlockTag::Pending),
        )
        .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                "failed to get tokenOfOwnerByIndex({address},{wallet},{index}) {:?}",
                e
            );
            return Err(ModelError::ProviderError(e));
        }
    };

    Ok(felt_to_u256(response.first().unwrap().to_owned()))
}

pub(crate) async fn get_value_of_token_in_slot(
    provider: &JsonRpcClient<HttpTransport>,
    address: &str,
    token_id: &U256,
) -> Result<U256, ModelError> {
    let response = match provider
        .call(
            get_call_function(
                &FieldElement::from_hex_be(address).unwrap(),
                "value_of",
                vec![u256_to_felt(token_id), FieldElement::ZERO],
            ),
            &BlockId::Tag(BlockTag::Pending),
        )
        .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("failed to get value_of({address},{token_id}): {:?}", e);
            return Err(ModelError::ProviderError(e));
        }
    };
    Ok(felt_to_u256(response.first().unwrap().to_owned()))
}

/// Load ERC-721 portfolio from starknet data
pub async fn load_erc_721_portfolio(
    project: &ProjectWithMinterAndPaymentViewModel,
    wallet: &str,
) -> Result<Vec<Token>, ModelError> {
    let provider = get_starknet_rpc_from_env()?;
    let mut tokens = vec![];
    // balance
    let balance = get_balance_of(&provider, &project.address, wallet).await?;
    for token_index in 0..balance {
        // tokenOfOwnerByIndex(owner, index) -> tokenId
        let token_id = get_token_id(&provider, &project.address, wallet, &token_index).await?;
        // tokenURI
        tokens.push(Token {
            token_id,
            name: project.name.to_owned(),
        });
    }

    Ok(tokens)
}

/// Load ERC-3525 portfolio from starknet data
pub async fn load_erc_3525_portfolio(
    project: &ProjectWithMinterAndPaymentViewModel,
    address: &str,
    slot: &U256,
    customer_tokens: &[CustomerToken],
) -> Result<Vec<Option<Erc3525Token>>, ModelError> {
    let provider = get_starknet_rpc_from_env()?;
    let mut tokens = vec![];
    for token_index in customer_tokens {
        if !(token_index.project_address == address && &token_index.slot == slot) {
            continue;
        }
        let value: U256 =
            get_value_of_token_in_slot(&provider, address, &token_index.token_id).await?;

        tokens.push(Some(Erc3525Token {
            token_id: token_index.token_id,
            name: project.name.to_string(),
            value,
            slot_value: SlotValue::from_blockchain(value, project.value_decimals).into(),
        }));
    }

    Ok(tokens)
}
