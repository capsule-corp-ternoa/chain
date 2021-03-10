#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::weights::Weight;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
    traits::{Currency, ExistenceRequirement},
};
use frame_system::ensure_signed;
use ternoa_common::traits::{LockableNFTs, NFTs};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod default_weights;

#[cfg(test)]
mod tests;

pub trait WeightInfo {
    fn list() -> Weight;
    fn unlist() -> Weight;
    fn buy() -> Weight;
}

pub trait Trait: frame_system::Trait {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    /// Currency used to handle transactions and pay for the nfts.
    type Currency: Currency<Self::AccountId>;
    /// Pallet managing nfts.
    type NFTs: LockableNFTs<AccountId = Self::AccountId>
        + NFTs<AccountId = Self::AccountId, NFTId = NFTIdOf<Self>>;
    /// Weight values for this pallet
    type WeightInfo: WeightInfo;
}

type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

type NFTIdOf<T> = <<T as Trait>::NFTs as LockableNFTs>::NFTId;

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
        BNftsalance = BalanceOf<T>,
        NFTId = NFTIdOf<T>,
    {
        /// A nft has been listed for sale. \[nft id, price\]
        NftListed(NFTId, BNftsalance),
        /// A nft is removed from the marketplace by its owner. \[nft id\]
        NftUnlisted(NFTId),
        /// A nft has been sold. \[nft id, new owner\]
        NftSold(NFTId, AccountId),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as Marketplace {
        /// Nfts listed on the marketplace
        pub NFTsForSale get(fn nft_for_sale): map hasher(blake2_128_concat) NFTIdOf<T> => (T::AccountId, BalanceOf<T>);
    }
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// This function is reserved to the owner of a nft.
        NotNftOwner,
        /// Nft is not present on the marketplace
        NftNotForSale,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Deposit a nft and list it on the marketplace
        #[weight = T::WeightInfo::list()]
        fn list(origin, nft_id: NFTIdOf<T>, price: BalanceOf<T>) {
            let who = ensure_signed(origin)?;
            ensure!(T::NFTs::owner(nft_id) == who, Error::<T>::NotNftOwner);

            T::NFTs::lock(nft_id)?;
            NFTsForSale::<T>::insert(nft_id, (who.clone(), price));

            Self::deposit_event(RawEvent::NftListed(nft_id, price));
        }

        /// Owner unlist the nfts
        #[weight = T::WeightInfo::unlist()]
        fn unlist(origin, nft_id: NFTIdOf<T>) {
            let who = ensure_signed(origin)?;

            ensure!(T::NFTs::owner(nft_id) == who, Error::<T>::NotNftOwner);
            ensure!(NFTsForSale::<T>::contains_key(nft_id), Error::<T>::NftNotForSale);

            T::NFTs::unlock(nft_id);
            NFTsForSale::<T>::remove(nft_id);

            Self::deposit_event(RawEvent::NftUnlisted(nft_id));
        }

        /// Buy a listed nft
        #[weight = T::WeightInfo::buy()]
        fn buy(origin, nft_id: NFTIdOf<T>) {
            let who = ensure_signed(origin)?;
            ensure!(NFTsForSale::<T>::contains_key(nft_id), Error::<T>::NftNotForSale);

            let (owner, price) = NFTsForSale::<T>::get(nft_id);
            // KeepAlive because they need to be able to use the NFT later on
            T::Currency::transfer(&who, &owner, price, ExistenceRequirement::KeepAlive)?;
            T::NFTs::set_owner(nft_id, &who)?;

            Self::deposit_event(RawEvent::NftSold(nft_id, who));
        }
    }
}
