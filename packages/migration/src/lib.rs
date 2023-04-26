pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table;
mod m20230420_072545_add_customer_tokens;
mod m20230420_075445_add_event_store;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20230420_072545_add_customer_tokens::Migration),
            Box::new(m20230420_075445_add_event_store::Migration),
        ]
    }
}
