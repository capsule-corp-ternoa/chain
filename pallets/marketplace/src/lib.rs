#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
    traits::{Currency, ExistenceRequirement},
};
use frame_system::ensure_signed;
use ternoa_common::traits::{CapsuleCreationEnabled, CapsuleTransferEnabled};

pub trait Trait: frame_system::Trait {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    /// Currency used to handle transactions and pay for the capsules.
    type Currency: Currency<Self::AccountId>;
    /// Pallet managing capsules.
    type Capsules: CapsuleTransferEnabled<AccountId = Self::AccountId>
        + CapsuleCreationEnabled<AccountId = Self::AccountId, CapsuleID = CapsuleIDOf<Self>>;
}

type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
type CapsuleIDOf<T> = <<T as Trait>::Capsules as CapsuleTransferEnabled>::CapsuleID;

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
        Balance = BalanceOf<T>,
        CapsuleID = CapsuleIDOf<T>,
    {
        /// A capsule has been listed for sale. \[capsule id, price\]
        CapsuleListed(CapsuleID, Balance),
        /// A capusle is removed from the marketplace by its owner. \[capsule id\]
        CapsuleUnlisted(CapsuleID),
        /// A capsule has been sold. \[capsule id, new owner\]
        CapsuleSold(CapsuleID, AccountId),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as Marketplace {
        /// Capsules listed on the marketplace
        pub CapsulesForSale get(fn capules_for_sale): map hasher(blake2_128_concat) CapsuleIDOf<T> => (T::AccountId, BalanceOf<T>);
    }
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// This function is reserved to the owner of a capsule.
        NotCapsuleOwner,
        /// Capsule is not present on the marketplace
        CapsuleNotForSale,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Deposit a capsule and list it on the marketplace
        #[weight = 0]
        fn list(origin, capsule_id: CapsuleIDOf<T>, price: BalanceOf<T>) {
            let who = ensure_signed(origin)?;
            ensure!(T::Capsules::is_owner(who.clone(), capsule_id), Error::<T>::NotCapsuleOwner);

            T::Capsules::lock(capsule_id)?;
            CapsulesForSale::<T>::insert(capsule_id, (who.clone(), price));

            Self::deposit_event(RawEvent::CapsuleListed(capsule_id, price));
        }

        /// Owner unlist the capsules
        #[weight = 0]
        fn unlist(origin, capsule_id: CapsuleIDOf<T>) {
            let who = ensure_signed(origin)?;
            ensure!(T::Capsules::is_owner(who.clone(), capsule_id), Error::<T>::NotCapsuleOwner);
            ensure!(CapsulesForSale::<T>::contains_key(capsule_id), Error::<T>::CapsuleNotForSale);

            T::Capsules::unlock(capsule_id)?;
            CapsulesForSale::<T>::remove(capsule_id);

            Self::deposit_event(RawEvent::CapsuleUnlisted(capsule_id));
        }

        /// Buy a listed capsule
        #[weight = 0]
        fn buy(origin, capsule_id: CapsuleIDOf<T>) {
            let who = ensure_signed(origin)?;
            ensure!(CapsulesForSale::<T>::contains_key(capsule_id), Error::<T>::CapsuleNotForSale);

            let (owner, price) = CapsulesForSale::<T>::get(capsule_id);
            // KeepAlive because they need to be able to use the NFT later on
            T::Currency::transfer(&who, &owner, price, ExistenceRequirement::KeepAlive)?;
            T::Capsules::transfer_from(owner.clone(), who.clone(), capsule_id)?;

            Self::deposit_event(RawEvent::CapsuleSold(capsule_id, who));
        }
    }
}
