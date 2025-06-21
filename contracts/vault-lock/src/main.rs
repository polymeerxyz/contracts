#![cfg_attr(not(any(feature = "library", test)), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(any(feature = "library", test))]
extern crate alloc;

#[cfg(not(any(feature = "library", test)))]
ckb_std::entry!(program_entry);
#[cfg(not(any(feature = "library", test)))]
// By default, the following heap configuration is used:
// * 16KB fixed heap
// * 1.2MB(rounded up to be 16-byte aligned) dynamic heap
// * Minimal memory block in dynamic heap is 64 bytes
// For more details, please refer to ckb-std's default_alloc macro
// and the buddy-alloc alloc implementation.
ckb_std::default_alloc!(16384, 1258306, 64);

use ckb_std::{
    ckb_constants::Source,
    debug,
    high_level::{load_cell_lock_hash, load_script, QueryIter},
};
use vault_lock::error::{BizError, Error};

pub fn program_entry() -> i8 {
    match entry() {
        Ok(()) => 0,
        Err(err) => err.into(),
    }
}

fn entry() -> Result<(), Error> {
    debug!("vault lock contract is executing");

    let args = load_script()?.args();
    let args_bytes = args.raw_data();

    if args_bytes.len() != 64 {
        Err(BizError::ArgumentLengthInvalid)?;
    }

    let creator_lock_hash = &args_bytes[0..32];
    let admin_lock_hash = &args_bytes[32..64];

    let mut creator_signed = false;
    let mut admin_signed = false;

    // Check if the transaction is signed by the creator or admin by looking
    // for an input cell with a matching lock hash.
    // We skip the first input, which is the vault cell itself.
    for i in 1..QueryIter::new(load_cell_lock_hash, Source::Input).count() {
        let lock_hash = load_cell_lock_hash(i, Source::Input)?;
        if lock_hash == creator_lock_hash {
            creator_signed = true;
        }
        if lock_hash == admin_lock_hash {
            admin_signed = true;
        }
    }

    if admin_signed {
        // Admin can only perform the distribution action.
        // The vault-type script will verify that this is a valid distribution.
        // If creator also signed, we let admin action take precedence.
        debug!("Admin action authorized");
        return Ok(());
    }

    if creator_signed {
        // Creator can perform refund or capacity adjustment actions.
        // The vault-type script will verify the specifics of the action.
        debug!("Creator action authorized");
        return Ok(());
    }

    Err(BizError::UnauthorizedAction.into())
}
