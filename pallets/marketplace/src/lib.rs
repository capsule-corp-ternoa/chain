#![cfg_attr(not(feature = "std"), no_std)]

use codec::Encode;
use frame_support::{
    decl_error, decl_event, decl_module, ensure,
    traits::{
        schedule::{DispatchTime, Named as ScheduleNamed},
        LockIdentifier,
    },
    Parameter,
};
use frame_system::{ensure_root, ensure_signed, RawOrigin};
use sp_runtime::{traits::Dispatchable, traits::StaticLookup, DispatchResult};
use ternoa_common::traits::{
    CapsuleCreationEnabled, CapsuleDefaultBuilder, CapsuleTransferEnabled,
};

/// Used for derivating scheduled tasks IDs
const ESCROW_ID: LockIdentifier = *b"escrow  ";

pub trait Trait: frame_system::Trait {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
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

type CapsuleIDOf<T> = <<T as Trait>::Capsules as CapsuleTransferEnabled>::CapsuleID;

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
        CapsuleID = CapsuleIDOf<T>,
        BlockNumber = <T as frame_system::Trait>::BlockNumber,
    {
        /// A transfer has been scheduled. \[capsule id, destination, block of transfer\]
        TransferScheduled(CapsuleID, AccountId, BlockNumber),
        /// A transfer has been canceled. \[capsule id\]
        TransferCanceled(CapsuleID),
        /// A transfer was executed and finalized. \[capsule id\]
        TransferCompleted(CapsuleID),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// This function is reserved to the owner of a capsule.
        NotCapsuleOwner,
        /// An unknown error happened which made the scheduling call fail.
        SchedulingFailed,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Create a timed transfer. This will lock the associated capsule until it gets
        /// transferred or canceled.
        #[weight = 0]
        fn create(origin, capsule_id: CapsuleIDOf<T>, to: <T::Lookup as StaticLookup>::Source, at: T::BlockNumber) {
            let who = ensure_signed(origin)?;
            Self::ensure_capsule_owner(who.clone(), capsule_id)?;
            let to_unlookup = T::Lookup::lookup(to)?;

            T::Capsules::lock(capsule_id)?;
            ensure!(T::Scheduler::schedule_named(
                (ESCROW_ID, capsule_id).encode(),
                DispatchTime::At(at),
                None,
                // priority was chosen arbitrarily, we made sure it is lower than runtime
                // upgrades and democracy calls
                100,
                RawOrigin::Root.into(),
                Call::complete_transfer(who, to_unlookup.clone(), capsule_id).into()
            ).is_ok(), Error::<T>::SchedulingFailed);

            Self::deposit_event(RawEvent::TransferScheduled(capsule_id, to_unlookup, at));
        }

        /// Cancel a transfer that was previously created and unlocks the capsule.
        #[weight = 0]
        fn cancel(origin, capsule_id: CapsuleIDOf<T>) {
            let who = ensure_signed(origin)?;
            Self::ensure_capsule_owner(who.clone(), capsule_id)?;

            ensure!(T::Scheduler::cancel_named((ESCROW_ID, capsule_id).encode()).is_ok(), Error::<T>::SchedulingFailed);
            T::Capsules::unlock(capsule_id)?;

            Self::deposit_event(RawEvent::TransferCanceled(capsule_id));
        }

        /// System only. Execute a transfer, called by the scheduler.
        #[weight = 0]
        fn complete_transfer(origin, from: T::AccountId, to: T::AccountId, capsule_id: CapsuleIDOf<T>) {
            ensure_root(origin)?;
            T::Capsules::unlock(capsule_id)?;
            T::Capsules::transfer_from(from, to, capsule_id)?;

            Self::deposit_event(RawEvent::TransferCompleted(capsule_id));
        }
    }
}

impl<T: Trait> Module<T> {
    fn ensure_capsule_owner(
        maybe_owner: T::AccountId,
        capsule_id: CapsuleIDOf<T>,
    ) -> DispatchResult {
        ensure!(
            T::Capsules::is_owner(maybe_owner, capsule_id),
            Error::<T>::NotCapsuleOwner
        );
        Ok(())
    }
}
