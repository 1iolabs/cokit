use crate::repository::Repository;

#[derive(Debug)]
pub struct Service {
    repo: Repository,
    db: db::DB,
    storage: Arc<dyn drivers::Storage>,
    kernel_service: Arc<kernel::Service>,
}
