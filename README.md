
## Add Reward Beneficiary Delegation

## Overview
To send the accumulated rewards to other accounts, stakers can now change the reward destination to DelegateBalance using `set_reward_destination` and then add delegatee_address using `set_delegatee` extrinsic. This must be done for all contract_id for which staker expect to receive rewards.

**WARNING: Once a staker changes RewardDestination from DelegateBalance to other, its delegatee_account information is deleted for all contract_ids and must be set up again if switched back to DelegateBalance.**

## Modified Data Structure
In order to delegate collected rewards to Beneficiary in DappStaking, we introduced **DelegateBalance** variant in `RewardDestination` 
```
pub enum RewardDestination {
    /// Rewards are transferred to stakers free balance without any further action.
    FreeBalance,
    /// Rewards are transferred to stakers balance and are immediately re-staked
    /// on the contract from which the reward was received.
    StakeBalance,
    /// Rewards are transfered to delegatee account's free balance
    DelegateBalance,
}
```
Keeps track of DelegateInfo 
```
 /// Info about delegatee address for (delegtor_address,contract_id) pair
    #[pallet::storage]
    #[pallet::getter(fn delegate_info)]
    pub type DelegateInfo<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        T::SmartContract,
        T::AccountId,
        OptionQuery,
    >;
```

## Alternate Approach
The given solution works fine but we can go a step further and make code even more sophisticated by allowing users to set up `RewardDestination` per contract instead of per `AccountId`.

### Modification Required:  Data Structures
#### AccountLedger
```
pub struct AccountLedger<Balance: AtLeast32BitUnsigned + Default + Copy + MaxEncodedLen, T: Config> {
    /// Total balance locked.
    #[codec(compact)]
    pub locked: Balance,
    /// Information about unbonding chunks.
    unbonding_info: UnbondingInfo<Balance>,
    /// Instruction on how to handle reward payout
    reward_destination_info: RewardDestintionInfo<T>,
}
```
#### RewardDestinationInfo
```
RewardDestinationInfo<T:Config> {
    /// reward destination per contract
    info: Vec<(T::SmartContract, RewardDestionation)>
}
```

```
impl<T: Config> RewardDestinationInfo<T> {
    // used to get the reward destination for a particular contract 
    pub fn get_reward_destination(&self, contract_id: T::SmartContract) -> RewardDestination {
        for i in 0..self.info.len() {
            if self.info[i].0 == contract_id {
                return self.info[i].1;
           
        }
    }
    // if RewardDestination not set, returns the default
        return RewardDestination::default();
            
    }
     
    pub fn add_reward_destination(
        &self,
        contract_id: T::SmartContract,
        destination: RewardDestination,
    ) {
        // find if the location exists for that particular destination
        for i in 0..self.info.len() {
            if self.info[i].0 == contract_id {
                self.info[i].1 = destination;
            }
        }
        
        // if no location, then pushed into info 
        self.info.push((contract_id, destination));
    }
}
```
