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

    // TODO: ensure inner receiver type is Self
    let ty = as_variant!(&*recv.ty, Type::Path)
        .map(|ty| &ty.path)
        // Extract the inner type of `Pin<T>`
        .filter(|path| is_std(path, "core", "pin", "Pin"))
        .and_then(|path| extract_inner_type(path))
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
            // Ensure `Self` is the only type argument
            if extract_inner_type(&p.path)
                .and_then(|ty| as_variant!(ty, Type::Path))
                .and_then(|p| p.path.get_ident())
                .map_or(true, |i| i != "Self")
            {
                return None;
            }
            if is_std(&p.path, "alloc", "boxed", "Box") {
                maybe_pinned!(BoxSelf)
            } else if is_std(&p.path, "alloc", "rc", "Rc") {
                maybe_pinned!(RcSelf)
            } else if is_std(&p.path, "alloc", "sync", "Arc") {
                maybe_pinned!(ArcSelf)
            } else {
                return None;
            }
        },
        _ => return None,
    };

    Some(Ident::new(sealed, recv.self_token.span))
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::quote;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(quote!(&self), Some("RefSelf"))]
    #[case(quote!(&mut self), Some("RefMutSelf"))]
    #[case(quote!(self: magic!(Self)), None)]
    #[case(quote!(self: Box<Self>), Some("BoxSelf"))]
    #[case(quote!(self: Box<Self, MyAllocator>), None)]
    #[case(quote!(self: std::boxed::Box<Self>), Some("BoxSelf"))]
    #[case(quote!(self: alloc::boxed::Box<Self>), Some("BoxSelf"))]
    #[case(quote!(self: fakestd::boxed::Box<Self>), None)]
    #[case(quote!(self: Rc<Self>), Some("RcSelf"))]
    #[case(quote!(self: std::rc::Rc<Self>), Some("RcSelf"))]
    #[case(quote!(self: alloc::rc::Rc<Self>), Some("RcSelf"))]
    #[case(quote!(self: std::fakerc::Rc<Self>), None)]
    #[case(quote!(self: Arc<Self>), Some("ArcSelf"))]
    #[case(quote!(self: std::sync::Arc<Self>), Some("ArcSelf"))]
    #[case(quote!(self: alloc::sync::Arc<Self>), Some("ArcSelf"))]
    #[case(quote!(self: std::sync::FakeArc<Self>), None)]
    #[case(quote!(self: std::sync::Arc<FakeSelf>), None)]
    #[case(quote!(self: Pin<&Self>), Some("PinRefSelf"))]
    #[case(quote!(self: std::pin::Pin<Box<Self>>), Some("PinBoxSelf"))]
    #[case(quote!(self: core::pin::Pin<&mut Self>), Some("PinRefMutSelf"))]
    fn inferred_receiver(#[case] recv: TokenStream, #[case] expected: Option<&str>) {
        let recv: syn::Receiver = syn::parse2(recv).unwrap();
        let result = infer_receiver(&recv);
        let expected = expected.map(|i| Ident::new(i, proc_macro2::Span::call_site()));
        assert_eq!(result, expected);
    }
}
