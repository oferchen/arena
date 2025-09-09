use migration::{Migrator, MigratorTrait};

#[test]
fn migrator_contains_email_otps() {
    let migrations = Migrator::migrations();
    let names: Vec<&str> = migrations.iter().map(|m| m.name()).collect();
    assert!(names.contains(&"m0004_email_otps"));
}
