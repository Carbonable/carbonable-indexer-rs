use std::collections::HashMap;

use apibara_core::starknet::v1alpha2::{FieldElement, Filter, HeaderFilter};
use apibara_sdk::Configuration;
use carbonable_domain::infrastructure::{app::Args, seed::read_data_content};
use starknet::macros::selector;

use crate::IndexerError;

/// Configure contract item filter
fn configure_filter_item(filter: &mut Filter, event_keys: Option<Vec<FieldElement>>, value: &str) {
    let keys = event_keys.unwrap_or(vec![]);
    filter.add_event(|e| {
        e.with_from_address(FieldElement::from_hex(value).unwrap())
            .with_keys(keys.to_vec())
    });
}

/// Get the base event configurators.
/// Might want to split it later on.
/// TODO: Find a more convenient way to build this
fn build_event_configurators() -> HashMap<String, Vec<FieldElement>> {
    let mut configurators = HashMap::new();
    let upgraded_selector = FieldElement::from_bytes(&selector!("Upgraded").to_bytes_be());

    configurators.insert(
        "project".into(),
        [
            upgraded_selector.clone(),
            FieldElement::from_bytes(&selector!("AbsorptionUpdate").to_bytes_be()),
            FieldElement::from_bytes(&selector!("Transfer").to_bytes_be()),
            FieldElement::from_bytes(&selector!("TransferValue").to_bytes_be()),
            FieldElement::from_bytes(&selector!("ApprovalValue").to_bytes_be()),
            FieldElement::from_bytes(&selector!("Approval").to_bytes_be()),
            FieldElement::from_bytes(&selector!("ApprovalForAll").to_bytes_be()),
            FieldElement::from_bytes(&selector!("SlotChanged").to_bytes_be()),
            // token_id metadata updated
            FieldElement::from_bytes(&selector!("MetadataUpdate").to_bytes_be()),
            // batch token_id metadata update
            FieldElement::from_bytes(&selector!("BatchMetadataUpdate").to_bytes_be()),
        ]
        .to_vec(),
    );
    configurators.insert(
        "minter".into(),
        [
            upgraded_selector.clone(),
            FieldElement::from_bytes(&selector!("AirDrop").to_bytes_be()),
            FieldElement::from_bytes(&selector!("Buy").to_bytes_be()),
            FieldElement::from_bytes(&selector!("PreSaleOpen").to_bytes_be()),
            FieldElement::from_bytes(&selector!("PreSaleClosed").to_bytes_be()),
            FieldElement::from_bytes(&selector!("PublicSaleOpen").to_bytes_be()),
            FieldElement::from_bytes(&selector!("PublicSaleClosed").to_bytes_be()),
            FieldElement::from_bytes(&selector!("SoldOut").to_bytes_be()),
            // https://github.com/carbonable-labs/cairo-migrator/blob/main/src/migrator/library.cairo#L28
            FieldElement::from_bytes(&selector!("Migration").to_bytes_be()),
        ]
        .to_vec(),
    );
    configurators.insert(
        "yielder".into(),
        [
            upgraded_selector.clone(),
            FieldElement::from_bytes(&selector!("Deposit").to_bytes_be()),
            FieldElement::from_bytes(&selector!("Withdraw").to_bytes_be()),
            FieldElement::from_bytes(&selector!("Snapshot").to_bytes_be()),
            FieldElement::from_bytes(&selector!("Vesting").to_bytes_be()),
        ]
        .to_vec(),
    );
    configurators.insert(
        "offseter".into(),
        [
            upgraded_selector.clone(),
            FieldElement::from_bytes(&selector!("Deposit").to_bytes_be()),
            FieldElement::from_bytes(&selector!("Withdraw").to_bytes_be()),
            FieldElement::from_bytes(&selector!("Claim").to_bytes_be()),
        ]
        .to_vec(),
    );
    configurators.insert("vester".into(), [upgraded_selector.clone()].to_vec());

    configurators
}

/// Configure stream filters.
pub fn configure_stream_filters<P: AsRef<std::path::Path>>(
    app_config: &Args,
    file_path: P,
) -> Result<Configuration<Filter>, IndexerError> {
    let content = read_data_content(file_path)?;
    let configurators = build_event_configurators();

    let config = Configuration::<Filter>::default()
        .with_starting_block(app_config.starting_block.into())
        .with_batch_size(app_config.batch_size.into())
        .with_filter(|mut filter| {
            filter.with_header(HeaderFilter::weak());
            content.iter().flatten().for_each(|(key, value)| {
                let event_keys = configurators.get(key);
                configure_filter_item(&mut filter, event_keys.cloned(), &value);
            });
            filter.build()
        });

    Ok(config)
}
