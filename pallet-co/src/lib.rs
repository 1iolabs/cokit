#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

mod library;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
    use crate::library::ListReference;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use libipld::cbor::DagCborCodec;
    use libipld::multihash::Code::Sha2_256;
    use libipld::multihash::MultihashDigest;
    use libipld::prelude::*;
    use libipld::Cid;
    use sp_std::prelude::*;

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    // The pallet's runtime storage items.
    // https://docs.substrate.io/v3/runtime/storage
    #[pallet::storage]
    #[pallet::getter(fn references)]
    // Learn more about declaring storage items:
    // https://docs.substrate.io/v3/runtime/storage#declaring-storage-items
    pub type References<T> = StorageMap<_, Blake2_128Concat, Vec<u8>, Vec<u8>, ValueQuery>;

    // Pallets use events to inform users when important changes are made.
    // https://docs.substrate.io/v3/runtime/events-and-errors
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Push CID reference. [Key, Value]
        PushReference(Vec<u8>, Vec<u8>),
        /// Set/Update CID reference. [Key, Value]
        SetReference(Vec<u8>, Vec<u8>),
        /// Remove CID reference. [Key]
        RemoveReference(Vec<u8>),
        /// Get CID reference. [Value]
        GetReference(Vec<u8>),
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// No reference has been set for given key.
        None,

        /// The provided reference could not be parses as CID.
        InvalidReference,

        /// The reference at the provided key could not be parsed as an CID.
        InvalidKey,

        /// Generic encoding error.
        GenericEncoding,
    }

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Push reference to list.
        #[pallet::call_index(0)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1, 1).ref_time())]
        pub fn push_reference(
            origin: OriginFor<T>,
            key: Vec<u8>,
            reference: Vec<u8>,
        ) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://docs.substrate.io/v3/runtime/origins
            ensure_signed(origin)?;

            // get list reference
            let reference_as_cid =
                Cid::try_from(reference).map_err(|e| Error::<T>::InvalidReference)?;
            let list_reference = match <References<T>>::try_get(&key) {
                Ok(next) => ListReference {
                    version: Default::default(),
                    reference: reference_as_cid,
                    next: Some(Cid::try_from(next).map_err(|e| Error::<T>::InvalidKey)?),
                },
                Err(_) => ListReference {
                    version: Default::default(),
                    reference: reference_as_cid,
                    next: None,
                },
            };

            // encode list reference
            let data = serde_ipld_dagcbor::to_vec(&list_reference)
                .map_err(|e| Error::<T>::GenericEncoding)?;
            let hash = Sha2_256.digest(&data);
            let cid = Cid::new_v1(DagCborCodec.into(), hash);
            let value = cid.to_bytes();

            // apply offchain index (CID => DATA)
            sp_io::offchain_index::set(&value, &data);

            // apply onchain state
            <References<T>>::insert(&key, &value);

            // Emit an event.
            Self::deposit_event(Event::PushReference(key, value));

            // Return a successful DispatchResultWithPostInfo
            Ok(())
        }

        /// Set an reference.
        #[pallet::call_index(1)]
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
        pub fn set_reference(origin: OriginFor<T>, key: Vec<u8>, value: Vec<u8>) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://docs.substrate.io/v3/runtime/origins
            ensure_signed(origin)?;

            // Update storage.
            <References<T>>::insert(&key, &value);

            // Emit an event.
            Self::deposit_event(Event::SetReference(key, value));

            // Return a successful DispatchResultWithPostInfo
            Ok(())
        }

        /// Get an reference.
        #[pallet::call_index(2)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads(1).ref_time())]
        pub fn get_reference(origin: OriginFor<T>, key: Vec<u8>) -> DispatchResult {
            ensure_signed(origin)?;

            // Validate value exists.
            ensure!(<References<T>>::contains_key(&key), Error::<T>::None);

            // Read a value from storage.
            let value = <References<T>>::get(key);

            // Emit an event.
            Self::deposit_event(Event::GetReference(value));

            // Return a successful DispatchResultWithPostInfo
            Ok(())
        }

        /// Remove an reference.
        #[pallet::call_index(3)]
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
        pub fn remove_reference(origin: OriginFor<T>, key: Vec<u8>) -> DispatchResult {
            ensure_signed(origin)?;

            // Validate value exists.
            ensure!(<References<T>>::contains_key(&key), Error::<T>::None);

            // Read a value from storage.
            <References<T>>::remove(&key);

            // Emit an event.
            Self::deposit_event(Event::RemoveReference(key));

            // Return a successful DispatchResultWithPostInfo
            Ok(())
        }
    }
}
