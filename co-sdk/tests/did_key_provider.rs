// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use co_sdk::{
	Application, ApplicationBuilder, CoReducer, DidKeyIdentity, DidKeyProvider, Identity, IdentityResolverError,
	PrivateIdentityResolver, CO_CORE_NAME_KEYSTORE,
};

async fn setup(application_identifier: &str) -> (Application, CoReducer, DidKeyProvider) {
	let application = ApplicationBuilder::new_memory(application_identifier.to_owned())
		.without_keychain()
		.with_disabled_feature("co-local-encryption")
		.build()
		.await
		.expect("application");
	let local_co = application.local_co_reducer().await.expect("local CO");
	let provider = DidKeyProvider::new(local_co.clone(), CO_CORE_NAME_KEYSTORE);
	(application, local_co, provider)
}

async fn insert(application: &Application, local_co: &CoReducer, key: co_core_keystore::Key) {
	local_co
		.push(&application.local_identity(), CO_CORE_NAME_KEYSTORE, &co_core_keystore::KeyStoreAction::Set(key))
		.await
		.expect("insert key");
}

#[tokio::test]
async fn mismatched_uri_is_rejected() {
	let (application, local_co, provider) = setup("did-key-provider-mismatch").await;
	let did_a = DidKeyIdentity::generate(Some(&[1; 32]));
	let did_b = DidKeyIdentity::generate(Some(&[2; 32]));
	let mut mislabeled_b = did_b.export().expect("export B");
	mislabeled_b.uri = did_a.identity().to_owned();
	insert(&application, &local_co, mislabeled_b).await;

	let error = provider
		.resolve_private(did_a.identity())
		.await
		.expect_err("mislabeled private key must be rejected");

	match error {
		IdentityResolverError::Other(error) => {
			let context = error.to_string();
			assert!(context.contains(did_a.identity()));
			assert!(context.contains(did_b.identity()));
		},
		IdentityResolverError::NotFound => panic!("stored mismatch must not become not-found"),
	}
}

#[tokio::test]
async fn matching_uri_resolves_requested_identity() {
	let (_application, _local_co, provider) = setup("did-key-provider-match").await;
	let did = DidKeyIdentity::generate(Some(&[3; 32]));
	provider.store(&did, None).await.expect("store matching key");

	let resolved = provider.resolve_private(did.identity()).await.expect("resolve matching key");

	assert_eq!(resolved.identity(), did.identity());
}

#[tokio::test]
async fn missing_uri_remains_not_found() {
	let (_application, _local_co, provider) = setup("did-key-provider-missing").await;
	let did = DidKeyIdentity::generate(Some(&[4; 32]));

	let result = provider.resolve_private(did.identity()).await;

	assert!(matches!(result, Err(IdentityResolverError::NotFound)));
}

#[tokio::test]
async fn malformed_key_remains_an_import_error() {
	let (application, local_co, provider) = setup("did-key-provider-malformed").await;
	let did = DidKeyIdentity::generate(Some(&[5; 32]));
	let mut malformed = did.export().expect("export key");
	malformed.tags = Default::default();
	insert(&application, &local_co, malformed).await;

	let error = provider
		.resolve_private(did.identity())
		.await
		.expect_err("malformed private key must fail import");

	match error {
		IdentityResolverError::Other(error) => {
			assert_eq!(error.to_string(), "Invalid identity format or key");
		},
		IdentityResolverError::NotFound => panic!("stored malformed key must not become not-found"),
	}
}
