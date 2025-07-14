use syn::spanned::Spanned;
use syn::{Ident, Type};

use crate::utils::*;

pub(crate) fn infer_receiver(recv: &syn::Receiver) -> Option<Ident> {
    let mut pinned = false;
    macro_rules! maybe_pinned {
        ($ty:ident) => {
            if pinned {
                concat!("Pin", stringify!($ty))
            } else {
                stringify!($ty)
            }
        };
    }

    let ty = as_variant!(Type::Path, &*recv.ty)
        .map(|ty| &ty.path)
        // Extract the inner type of `Pin<T>`
        .filter(|path| is_std(path, "pin", "Pin"))
        .map(|path| path.segments.last().unwrap())
        .and_then(|seg| extract_inner_type(&seg.arguments))
        .inspect(|_| pinned = true)
        .unwrap_or(&recv.ty);

    let sealed = match ty {
        Type::Reference(r) => {
            if r.mutability.is_none() {
                maybe_pinned!(RefSelf)
            } else {
                maybe_pinned!(RefMutSelf)
            }
        },
        Type::Path(p) => {
            if is_std(&p.path, "boxed", "Box") {
                maybe_pinned!(BoxSelf)
            } else if is_std(&p.path, "rc", "Rc") {
                maybe_pinned!(RcSelf)
            } else if is_std(&p.path, "arc", "Arc") {
                maybe_pinned!(ArcSelf)
            } else {
                return None;
            }
        },
        _ => return None,
    };

    Some(Ident::new(sealed, ty.span()))
}
