#![cfg_attr(not(feature = "std"), no_std)]

use codec::Encode;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
    traits::{
        schedule::{DispatchTime, Named as ScheduleNamed},
        Currency, LockIdentifier,
    },
    Parameter,
};
use frame_system::{ensure_root, ensure_signed, RawOrigin};
use sp_runtime::{traits::Dispatchable, traits::StaticLookup, DispatchResult};
use ternoa_common::traits::{
    CapsuleCreationEnabled, CapsuleDefaultBuilder, CapsuleTransferEnabled,
};

pub trait Trait: frame_system::Trait {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    /// Currency used to handle transactions and pay for the capsules.
    type Currency: Currency<Self::AccountId>;
    /// Pallet managing capsules.
    type Capsules: CapsuleTransferEnabled<AccountId = Self::AccountId>
        + CapsuleCreationEnabled<
            AccountId = Self::AccountId,
            CapsuleID = CapsuleIDOf<Self>,
            CapsuleData = Self::CapsuleData,
        >;
    /// How a capsule's data is represented. Mostly used for benchmarks when passed to the
    /// `CapsuleCreationEnabled` trait.
    type CapsuleData: Parameter + CapsuleDefaultBuilder<Self::AccountId>;
    /// Scheduler instance which we use to schedule actual transfer calls. This way, we have
    /// all scheduled calls accross all pallets in one place.
    type Scheduler: ScheduleNamed<Self::BlockNumber, Self::PalletsCall, Self::PalletsOrigin>;
    /// Overarching type of all pallets origins. Used with the scheduler.
    type PalletsOrigin: From<RawOrigin<Self::AccountId>>;
    /// Overarching type of all pallets calls. Used by the scheduler.
    type PalletsCall: Dispatchable<Origin = Self::Origin> + From<Call<Self>>;
}

type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
type CapsuleIDOf<T> = <<T as Trait>::Capsules as CapsuleTransferEnabled>::CapsuleID;

decl_event!(
    pub enum Event<T>
    where
        Balance = BalanceOf<T>,
        CapsuleID = CapsuleIDOf<T>,
    {
        /// A capsule has been listed for sale. \[capsule id, price\]
        CapsuleListed(CapsuleID, Balance),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as Marketplace {
        /// Capsules listed on the marketplace
        pub Capsules get(fn capules): map hasher(blake2_128_concat) CapsuleIDOf<T> => BalanceOf<T>;
    }
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// This function is reserved to the owner of a capsule.
        NotCapsuleOwner,
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
            Capsules::<T>::insert(capsule_id, price);

            Self::deposit_event(RawEvent::CapsuleListed(capsule_id, price));
        }
    }
}
