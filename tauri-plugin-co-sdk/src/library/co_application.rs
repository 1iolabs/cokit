use crate::library::cli::Cli;
use clap::Parser;
use co_core_co::CoAction;
use co_primitives::TagPattern;
use co_sdk::{Application, ApplicationBuilder, CoInvite, KnownTag, KnownTags, NetworkSettings, Tags, CO_CORE_NAME_CO};
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct CoApplicationSettings {
	pub instance_id: String,
	pub base_path: Option<PathBuf>,
	pub force_new_peer_id: bool,
	pub network: bool,
	pub no_keychain: bool,
	pub no_log: bool,
	pub auto_accept_invite: bool,
}
impl CoApplicationSettings {
	pub fn new(identifier: &str) -> Self {
		CoApplicationSettings { instance_id: identifier.into(), ..Default::default() }
	}

	/// Create `CoApplicationSettings` from command line args.
	pub fn cli(identifier: &str) -> Self {
		let mut cli = Cli::parse();
		if cli.instance_id.is_none() {
			cli.instance_id = Some(identifier.to_owned());
		}
		cli.into()
	}

	pub fn with_path(self, path: &str) -> Self {
		Self { base_path: Some(path.into()), ..self }
	}

	pub fn with_network(self, force_new_peer_id: bool) -> Self {
		Self { network: true, force_new_peer_id, ..self }
	}

	pub fn without_keychain(self) -> Self {
		Self { no_keychain: true, ..self }
	}
}

pub async fn application(settings: CoApplicationSettings) -> Application {
	let identifier = settings.instance_id;
	let mut builder = match settings.base_path {
		Some(path) => ApplicationBuilder::new_with_path(identifier, path),
		None => ApplicationBuilder::new(identifier),
	};
	if settings.no_keychain {
		builder = builder.without_keychain()
	}
	let mut application = builder.with_bunyan_logging(None).build().await.expect("application");

	// network
	if settings.network {
		application
			.create_network(NetworkSettings::new().with_force_new_peer_id(settings.force_new_peer_id))
			.await
			.expect("network");
	}

	let local_co = application.local_co_reducer().await.expect("local co");

	if settings.auto_accept_invite {
		// check current invite tag
		let insert_tag = match local_co
			.co()
			.await
			.expect("local co state")
			.1
			.tags
			.find_key(&KnownTags::CoInvite.to_string())
		{
			None => true,
			Some(tag) => {
				if tag.matches_pattern(&CoInvite::Accept.tag()) {
					// already set to 'accept'
					false
				} else {
					// remove old invite key tag if not accept
					let mut tags = Tags::new();
					tags.insert(tag.clone());
					local_co
						.push(&application.local_identity(), CO_CORE_NAME_CO, &CoAction::TagsRemove { tags })
						.await
						.expect("tag removed");
					true
				}
			},
		};

		if insert_tag {
			// add new invite key tag
			let mut tags = Tags::new();
			tags.insert(CoInvite::Accept.tag());
			local_co
				.push(&application.local_identity(), CO_CORE_NAME_CO, &CoAction::TagsInsert { tags })
				.await
				.expect("tag inserted");
		}
	} else {
		// remove current accept tag if present
		match local_co
			.co()
			.await
			.expect("local co state")
			.1
			.tags
			.find_key(&KnownTags::CoInvite.to_string())
		{
			None => (),
			Some(tag) => {
				if tag.matches_pattern(&CoInvite::Accept.tag()) {
					// remove
					let mut tags = Tags::new();
					tags.insert(CoInvite::Accept.tag());
					local_co
						.push(&application.local_identity(), CO_CORE_NAME_CO, &CoAction::TagsRemove { tags })
						.await
						.expect("tag removed");
				}
			},
		};
	}

	application.clone()
}
