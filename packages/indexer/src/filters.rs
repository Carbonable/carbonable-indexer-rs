use apibara_core::{
    node::v1alpha2::DataFinality,
    starknet::v1alpha2::{FieldElement, Filter, HeaderFilter},
};
use apibara_sdk::Configuration;
use carbonable_domain::{
    domain::event_source::Filterable,
    infrastructure::{app::Args, seed::read_data_content},
};

use crate::IndexerError;

/// Configure apibara stream contract item filter
/// * `filter` - The filter to configure
/// * `application_filter` - The application filter to use
///
#[allow(clippy::borrowed_box)]
fn configure_filter_item(filter: &mut Filter, application_filter: &Box<dyn Filterable>) {
    for af in application_filter.to_filters() {
        filter.add_event(|e| {
            e.with_from_address(FieldElement::from_hex(&af.0).unwrap())
                .with_keys([FieldElement::from_hex(&af.1).unwrap()].to_vec())
        });
    }
}

/// Configure stream filters for apibara
/// * `app_config` - The application configuration
/// * `file_path` - The path to the file containing the contract addresses
/// * `application_filters` - The application filters to use
/// * `last_block_id` - The last block id to start from
///
pub fn configure_stream_filters<P: AsRef<std::path::Path>>(
    app_config: &Args,
    file_path: P,
    application_filters: &mut [Box<dyn Filterable>],
    last_block_id: &u64,
) -> Result<Configuration<Filter>, IndexerError> {
    let content = read_data_content(file_path)?;

    for filter in application_filters.iter_mut() {
        filter.hydrate_from_file(content.clone());
    }

    let config = Configuration::<Filter>::default()
        .with_starting_block(*last_block_id)
        // .with_batch_size(app_config.batch_size)
        .with_finality(DataFinality::DataStatusPending)
        .with_filter(|mut filter| {
            filter.with_header(HeaderFilter::weak());
            for f in application_filters.iter() {
                configure_filter_item(&mut filter, f);
            }
            filter.build()
        });

    Ok(config)
}
