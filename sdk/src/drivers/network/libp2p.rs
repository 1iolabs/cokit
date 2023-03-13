use anyhow::Result;

#[derive(Debug)]
pub struct Libp2pNetwork {
}

#[derive(Clone, Debug)]
pub struct Libp2pNetworkConfig {
    pub base_path: PathBuf,
}

impl Libp2pNetwork {
    pub async fn new(config: Libp2pNetworkConfig) -> Result<Libp2pNetwork> {
        
    }
}
