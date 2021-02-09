#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, Parameter};
use frame_system::ensure_signed;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
    traits::{CheckedAdd, MaybeSerializeDeserialize, Member},
    RuntimeDebug,
};
use ternoa_common::traits::{LockableNFTs, NFTs};

#[cfg(test)]
mod tests;

/// Data related to an NFT, such as who is its owner.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct NFTData<AccountId, NFTDetails> {
    pub owner: AccountId,
    pub details: NFTDetails,
    /// Set to true to prevent further modifications to the details struct
    pub sealed: bool,
}

pub trait Trait: frame_system::Trait {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    /// How NFTs are represented
    type NFTId: Parameter + Default + CheckedAdd + Copy + Member + From<u8>;
    /// How NFT details are represented
    type NFTDetails: Parameter + Member + MaybeSerializeDeserialize + Default;
}

decl_storage! {
    trait Store for Module<T: Trait> as NFTs {
        /// The number of NFTs managed by this pallet
        pub Total get(fn total): T::NFTId;
        /// Data related to NFTs.
        pub Data get(fn data): map hasher(blake2_128_concat) T::NFTId => NFTData<T::AccountId, T::NFTDetails>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
        NFTId = <T as Trait>::NFTId,
    {
        /// A new NFT was created. \[nft id, owner\]
        Created(NFTId, AccountId),
        /// An NFT was transferred to someone else. \[nft id, old owner, new owner\]
        Transfer(NFTId, AccountId, AccountId),
        /// An NFT was updated by its owner. \[nft id\]
        Mutated(NFTId),
        /// An NFT was sealed, preventing any new mutations. \[nft id\]
        Sealed(NFTId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// We do not have any NFT id left, a runtime upgrade is necessary.
        NFTIdOverflow,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Create a new NFT with the provided details. An ID will be auto
        /// generated and logged as an event, The caller of this function
        /// will become the owner of the new NFT.
        #[weight = 0]
        fn create(origin, details: T::NFTDetails) {
            let who = ensure_signed(origin)?;
            let id = Total::<T>::get();
            Total::<T>::put(id.checked_add(&1.into()).ok_or(Error::<T>::NFTIdOverflow)?);

            Data::<T>::insert(id, NFTData {
                owner: who.clone(),
                details,
                sealed: false,
            });

            Self::deposit_event(RawEvent::Created(id, who));
        }

        // mutate(origin, id: NFTId, details: NFTDetails)
        // transfer(origin, id: NFTId, who: <T::Lookup as StaticLookup>::Source)
        // seal(origin, id: NFTId)
    }
}
