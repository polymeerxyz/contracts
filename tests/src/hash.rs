use ckb_testtool::{
    ckb_types::packed::{Byte32, OutPoint},
    context::Context,
};

pub fn get_code_hash(ctx: &mut Context, outpoint: &OutPoint) -> Byte32 {
    let lock_template = ctx.build_script(outpoint, Default::default()).unwrap();
    // This is the true code_hash that the context will use for this script.
    lock_template.code_hash()
}
