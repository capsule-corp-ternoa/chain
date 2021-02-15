#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
    traits::{Currency, ExistenceRequirement},
};
use frame_system::ensure_signed;
use ternoa_common::traits::{LockableNFTs, NFTs};

pub trait Trait: frame_system::Trait {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    /// Currency used to handle transactions and pay for the nfts.
    type Currency: Currency<Self::AccountId>;
    /// Pallet managing nfts.
    type Nfts: LockableNFTs<AccountId = Self::AccountId>;

    type Data: NFTs<Self::AccountId> as Data

}

type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
type NftIDOf<T> = <<T as Trait>::NFTs as LockableNFTs>::NFTId;

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
        Balance = BalanceOf<T>,
        NftID = NftIDOf<T>,
    {
        /// A nft has been listed for sale. \[nft id, price\]
        NftListed(NftID, Balance),
        /// A nft is removed from the marketplace by its owner. \[nft id\]
        NftUnlisted(NftID),
        /// A nft has been sold. \[nft id, new owner\]
        NftSold(NftID, AccountId),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as Marketplace {
        /// Nfts listed on the marketplace
        pub NftsForSale get(fn nft_for_sale): map hasher(blake2_128_concat) NftIDOf<T> => (T::AccountId, BalanceOf<T>);
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
        #[weight = 0]
        fn list(origin, nft_id: NftIDOf<T>, price: BalanceOf<T>) {
            let who = ensure_signed(origin)?;
            let mut data = <dyn NFTs as Trait>::Data::<T>::get(nft_id);

            ensure!(data.owner == &who, Error::<T>::NotNftOwner);

            T::NFTs::lock(nft_id)?;
            NFTsForSale::<T>::insert(nft_id, (who.clone(), price));

            Self::deposit_event(RawEvent::NftListed(nft_id, price));
        }

        /// Owner unlist the nfts
        #[weight = 0]
        fn unlist(origin, nft_id: NftIDOf<T>) {
            let who = ensure_signed(origin)?;
            let mut data = NFTs::Data::<T>::get(nft_id);

            ensure!(data.owner == &who, Error::<T>::NotNftOwner);
            ensure!(NftsForSale::<T>::contains_key(nft_id), Error::<T>::NFTNotForSale);

            T::Nfts::unlock(nft_id)?;
            NftsForSale::<T>::remove(nft_id);

            Self::deposit_event(RawEvent::NftUnlisted(nft_id));
        }

        /// Buy a listed nft
        #[weight = 0]
        fn buy(origin, nft_id: NftIDOf<T>) {
            let who = ensure_signed(origin)?;
            ensure!(NftsForSale::<T>::contains_key(nft_id), Error::<T>::NftNotForSale);

            let (owner, price) = NftsForSale::<T>::get(nft_id);
            // KeepAlive because they need to be able to use the NFT later on
            T::Currency::transfer(&who, &owner, price, ExistenceRequirement::KeepAlive)?;
            T::Nfts::transfer(owner.clone(), nft_id, who.clone())?;

            Self::deposit_event(RawEvent::NftSold(nft_id, who));
        }
    }
}
