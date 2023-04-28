pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table;
mod m20230420_072545_add_customer_tokens;
mod m20230420_075445_add_event_store;
mod m20230427_150756_add_yielder_event_source;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20230420_072545_add_customer_tokens::Migration),
            Box::new(m20230420_075445_add_event_store::Migration),
            Box::new(m20230427_150756_add_yielder_event_source::Migration),
        ]
    }
}
