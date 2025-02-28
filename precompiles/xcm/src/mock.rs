// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

//! Testing utilities.

use super::*;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
    construct_runtime, parameter_types,
    traits::{AsEnsureOriginWithArg, Everything},
    weights::Weight,
};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

use pallet_evm::{
    AddressMapping, EnsureAddressNever, EnsureAddressRoot, PrecompileResult, PrecompileSet,
};
use pallet_evm_precompile_assets_erc20::AddressToAssetId;
use sp_core::{ConstU32, H160, H256};
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};
use sp_std::{borrow::Borrow, cell::RefCell};

use xcm::prelude::XcmVersion;
use xcm_builder::{
    test_utils::TransactAsset, AllowKnownQueryResponses, AllowSubscriptionsFrom,
    AllowTopLevelPaidExecutionFrom, FixedWeightBounds, LocationInverter, SignedToAccountId32,
    TakeWeightCredit,
};
use xcm_executor::XcmExecutor;

pub type AccountId = TestAccount;
pub type AssetId = u128;
pub type Balance = u128;
pub type BlockNumber = u64;
pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
pub type Block = frame_system::mocking::MockBlock<Runtime>;

pub const PRECOMPILE_ADDRESS: H160 = H160::repeat_byte(0x7B);
pub const ASSET_PRECOMPILE_ADDRESS_PREFIX: &[u8] = &[255u8; 4];

#[derive(
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Clone,
    Encode,
    Decode,
    Debug,
    MaxEncodedLen,
    Serialize,
    Deserialize,
    derive_more::Display,
    TypeInfo,
)]
pub enum TestAccount {
    Alice,
    Bob,
    Charlie,
    Bogus,
    Precompile,
}

impl Default for TestAccount {
    fn default() -> Self {
        Self::Alice
    }
}

impl AddressMapping<TestAccount> for TestAccount {
    fn into_account_id(h160_account: H160) -> TestAccount {
        match h160_account {
            a if a == H160::repeat_byte(0xAA) => Self::Alice,
            a if a == H160::repeat_byte(0xBB) => Self::Bob,
            a if a == H160::repeat_byte(0xCC) => Self::Charlie,
            a if a == PRECOMPILE_ADDRESS => Self::Precompile,
            _ => Self::Bogus,
        }
    }
}

impl From<H160> for TestAccount {
    fn from(x: H160) -> TestAccount {
        TestAccount::into_account_id(x)
    }
}

impl From<TestAccount> for H160 {
    fn from(value: TestAccount) -> H160 {
        match value {
            TestAccount::Alice => H160::repeat_byte(0xAA),
            TestAccount::Bob => H160::repeat_byte(0xBB),
            TestAccount::Charlie => H160::repeat_byte(0xCC),
            TestAccount::Precompile => PRECOMPILE_ADDRESS,
            TestAccount::Bogus => Default::default(),
        }
    }
}

impl From<TestAccount> for [u8; 32] {
    fn from(value: TestAccount) -> [u8; 32] {
        match value {
            TestAccount::Alice => [0xAA; 32],
            TestAccount::Bob => [0xBB; 32],
            TestAccount::Charlie => [0xCC; 32],
            _ => Default::default(),
        }
    }
}

impl AddressToAssetId<AssetId> for Runtime {
    fn address_to_asset_id(address: H160) -> Option<AssetId> {
        let mut data = [0u8; 16];
        let address_bytes: [u8; 20] = address.into();
        if ASSET_PRECOMPILE_ADDRESS_PREFIX.eq(&address_bytes[0..4]) {
            data.copy_from_slice(&address_bytes[4..20]);
            Some(u128::from_be_bytes(data))
        } else {
            None
        }
    }

    fn asset_id_to_address(asset_id: AssetId) -> H160 {
        let mut data = [0u8; 20];
        data[0..4].copy_from_slice(ASSET_PRECOMPILE_ADDRESS_PREFIX);
        data[4..20].copy_from_slice(&asset_id.to_be_bytes());
        H160::from(data)
    }
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Runtime {
    type BaseCallFilter = Everything;
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type RuntimeCall = RuntimeCall;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type BlockWeights = ();
    type BlockLength = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

#[derive(Debug, Clone, Copy)]
pub struct TestPrecompileSet<R>(PhantomData<R>);

impl<R> PrecompileSet for TestPrecompileSet<R>
where
    R: pallet_evm::Config
        + pallet_xcm::Config
        + pallet_assets::Config
        + AddressToAssetId<<R as pallet_assets::Config>::AssetId>,
    XcmPrecompile<R, AssetIdConverter<AssetId>>: Precompile,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        match handle.code_address() {
            a if a == PRECOMPILE_ADDRESS => Some(
                XcmPrecompile::<R, AssetIdConverter<AssetId>>::execute(handle),
            ),
            _ => None,
        }
    }

    fn is_precompile(&self, address: H160) -> bool {
        address == PRECOMPILE_ADDRESS
    }
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 0;
}

impl pallet_balances::Config for Runtime {
    type MaxReserves = ();
    type ReserveIdentifier = ();
    type MaxLocks = ();
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

// These parameters dont matter much as this will only be called by root with the forced arguments
// No deposit is substracted with those methods
parameter_types! {
    pub const AssetDeposit: Balance = 0;
    pub const AssetAccountDeposit: Balance = 0;
    pub const ApprovalDeposit: Balance = 0;
    pub const AssetsStringLimit: u32 = 50;
    pub const MetadataDepositBase: Balance = 0;
    pub const MetadataDepositPerByte: Balance = 0;
}

impl pallet_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type AssetId = AssetId;
    type Currency = Balances;
    type ForceOrigin = frame_system::EnsureRoot<AccountId>;
    type AssetDeposit = AssetDeposit;
    type AssetAccountDeposit = AssetAccountDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = AssetsStringLimit;
    type Freezer = ();
    type Extra = ();
    type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId>>;
    type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
    type RemoveItemsLimit = ConstU32<0>;
    type AssetIdParameter = AssetId;
    type CallbackHandle = ();
}

pub struct AssetIdConverter<AssetId>(PhantomData<AssetId>);
impl<AssetId> xcm_executor::traits::Convert<MultiLocation, AssetId> for AssetIdConverter<AssetId>
where
    AssetId: Clone + Eq + From<u8>,
{
    fn convert_ref(id: impl Borrow<MultiLocation>) -> Result<AssetId, ()> {
        if id.borrow().eq(&MultiLocation::parent()) {
            Ok(AssetId::from(1u8))
        } else {
            Err(())
        }
    }
    fn reverse_ref(what: impl Borrow<AssetId>) -> Result<MultiLocation, ()> {
        if what.borrow().eq(&AssetId::from(1u8)) {
            Ok(MultiLocation::parent())
        } else {
            Err(())
        }
    }
}

parameter_types! {
    pub const PrecompilesValue: TestPrecompileSet<Runtime> =
        TestPrecompileSet(PhantomData);
    pub WeightPerGas: Weight = Weight::from_ref_time(1);
}

impl pallet_evm::Config for Runtime {
    type FeeCalculator = ();
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type CallOrigin = EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = EnsureAddressNever<AccountId>;
    type AddressMapping = AccountId;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type PrecompilesType = TestPrecompileSet<Self>;
    type PrecompilesValue = PrecompilesValue;
    type ChainId = ();
    type OnChargeTransaction = ();
    type BlockGasLimit = ();
    type BlockHashMapping = pallet_evm::SubstrateBlockHashMapping<Self>;
    type FindAuthor = ();
}

parameter_types! {
    pub const RelayLocation: MultiLocation = Here.into();
    pub const AnyNetwork: NetworkId = NetworkId::Any;
    pub Ancestry: MultiLocation = Here.into();
    pub UnitWeightCost: u64 = 1_000;
}

parameter_types! {
    pub const BaseXcmWeight: u64 = 1_000;
    pub const MaxInstructions: u32 = 100;
}

pub type Barrier = (
    TakeWeightCredit,
    AllowTopLevelPaidExecutionFrom<Everything>,
    AllowKnownQueryResponses<XcmPallet>,
    AllowSubscriptionsFrom<Everything>,
);

pub struct LocalAssetTransactor;

impl TransactAsset for LocalAssetTransactor {
    fn transfer_asset(
        _asset: &MultiAsset,
        _from: &MultiLocation,
        _to: &MultiLocation,
    ) -> Result<xcm_executor::Assets, XcmError> {
        Ok(xcm_executor::Assets::new())
    }
}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = StoringRouter;
    type AssetTransactor = LocalAssetTransactor;
    type OriginConverter = ();
    type IsReserve = ();
    type IsTeleporter = ();
    type LocationInverter = LocationInverter<Ancestry>;
    type Barrier = Barrier;
    type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
    type Trader = ();
    type ResponseHandler = XcmPallet;
    type AssetTrap = XcmPallet;
    type AssetClaims = XcmPallet;
    type SubscriptionService = XcmPallet;
}

parameter_types! {
    pub static AdvertisedXcmVersion: XcmVersion = 2;
}

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, AnyNetwork>;

thread_local! {
    pub static SENT_XCM: RefCell<Vec<(MultiLocation, Xcm<()>)>> = RefCell::new(Vec::new());
}

pub(crate) fn _sent_xcm() -> Vec<(MultiLocation, Xcm<()>)> {
    SENT_XCM.with(|q| (*q.borrow()).clone())
}

pub(crate) fn take_sent_xcm() -> Vec<(MultiLocation, Xcm<()>)> {
    SENT_XCM.with(|q| {
        let mut r = Vec::new();
        std::mem::swap(&mut r, &mut *q.borrow_mut());
        r
    })
}

pub struct StoringRouter;
impl SendXcm for StoringRouter {
    fn send_xcm(dest: impl Into<MultiLocation>, msg: Xcm<()>) -> SendResult {
        SENT_XCM.with(|q| q.borrow_mut().push((dest.into(), msg)));
        Ok(())
    }
}

impl pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type SendXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmRouter = StoringRouter;
    type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmExecuteFilter = Everything;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = Everything;
    type XcmReserveTransferFilter = Everything;
    type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
    type LocationInverter = LocationInverter<Ancestry>;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
    type AdvertisedXcmVersion = AdvertisedXcmVersion;
}

// Configure a mock runtime to test the pallet.
construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Balances: pallet_balances,
        Assets: pallet_assets,
        Evm: pallet_evm,
        Timestamp: pallet_timestamp,
        XcmPallet: pallet_xcm,
    }
);

#[derive(Default)]
pub(crate) struct ExtBuilder;

impl ExtBuilder {
    pub(crate) fn build(self) -> sp_io::TestExternalities {
        let t = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .expect("Frame system builds valid default genesis config");

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}
