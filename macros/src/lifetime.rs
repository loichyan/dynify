use std::collections::BTreeMap;

use proc_macro2::Span;
use quote::format_ident;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;
use syn::{parse_quote, parse_quote_spanned, visit_mut, FnArg, Ident, Lifetime, Result, Token};

pub(crate) struct TraitContext<'a> {
    pub generics: &'a syn::Generics,
}

pub(crate) fn inject_output_lifetime(
    context: Option<&TraitContext>,
    sig: &mut syn::Signature,
    output_lifetime: &Lifetime,
) -> Result<()> {
    // Collect lifetimes in the signature.
    let mut explicit = BTreeMap::from_iter(
        sig.generics
            .lifetimes()
            // Ignore lifetimes with attributes such as `#[cfg]`.
            .filter(|lt| lt.attrs.is_empty())
            .map(|lt| (lt.lifetime.clone(), false)),
    );
    let mut elided = Vec::new(); // reused across iterations
    for arg in sig.inputs.iter_mut() {
        let basename = match arg {
            FnArg::Receiver(recv) => Ident::new("this", recv.self_token.span),
            FnArg::Typed(a) => as_variant!(&*a.pat, syn::Pat::Ident)
                .map(|p| &p.ident)
                .ok_or_else(|| {
                    syn::Error::new_spanned(&a.pat, "typed argument must be a valid identifier")
                })?
                .clone(),
        };
        let mut coll = LifetimeCollector {
            basename: &basename,
            explicit: &mut explicit,
            elided: &mut elided,
            index: 0,
            state: Pass::First,
        };

        // In first pass, we assume there's only one elided lifetime and
        // transform the first elided lifetime into `'{basename}`.
        let first_index = coll.elided.len();
        coll.visit_fn_arg_mut(arg);
        // If more than one lifetime is found, we start the second pass to fix
        // the first elided lifetime, appending the missing index to it.
        if coll.index > 1 && coll.elided[first_index].ident == basename {
            coll.state = Pass::Second;
            coll.index = first_index; // used to locate the first lifetime
            coll.visit_fn_arg_mut(arg);
        }
    }

    // It doesn't matter the lifetiems are inserted after type or const params,
    // as lifetimes are always printed before them.
    let elided_params = elided
        .iter()
        .map::<syn::GenericParam, _>(|lt| parse_quote!(#lt))
        .chain(std::iter::once(parse_quote!(#output_lifetime)));
    sig.generics.params.extend(elided_params);

    // Ensure every lifetime outlives the output lifetime
    for lt in explicit
        .iter()
        .filter(|(_, selected)| **selected)
        .map(|(lt, _)| lt)
        .chain(elided.iter())
        .chain(
            context
                .into_iter()
                .flat_map(|c| c.generics.lifetimes())
                .map(|p| &p.lifetime),
        )
    {
        default_where_clause(&mut sig.generics.where_clause)
            .predicates
            .push(parse_quote_spanned! (lt.span() => #lt: #output_lifetime));
    }

    // Ensure every generic type outlives the output lifetime
    for param in sig
        .generics
        .params
        .iter()
        .chain(context.into_iter().flat_map(|c| c.generics.params.iter()))
    {
        let syn::GenericParam::Type(ty) = param else {
            continue;
        };
        default_where_clause(&mut sig.generics.where_clause)
            .predicates
            .push(parse_quote_spanned!(ty.span() => #ty: #output_lifetime));
    }

    // Ensure `Self` outlives the output lifetime
    if let Some(recv) = sig.receiver() {
        let span = recv.self_token.span;
        default_where_clause(&mut sig.generics.where_clause)
            .predicates
            .push(parse_quote_spanned!(span => Self: #output_lifetime));
    }

    Ok(())
}

struct LifetimeCollector<'a> {
    basename: &'a Ident,
    explicit: &'a mut BTreeMap<Lifetime, bool>,
    elided: &'a mut Vec<Lifetime>,
    index: usize,
    state: Pass,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Pass {
    First,
    Second,
    Finished,
}

impl LifetimeCollector<'_> {
    fn is_finished(&self) -> bool {
        self.state == Pass::Finished
    }

    fn collect_opt_lifetime(&mut self, span: Span, lifetime: &mut Option<Lifetime>) {
        match lifetime {
            Some(lt) => self.collect_lifetime(lt),
            None if !self.is_finished() => *lifetime = Some(self.next_lifetime(span)),
            None => unreachable!(),
        }
    }

    fn collect_lifetime(&mut self, lifetime: &mut Lifetime) {
        match self.state {
            Pass::First if lifetime.ident == "_" => *lifetime = self.next_lifetime(lifetime.span()),
            Pass::First => {
                self.index += 1;
                _ = self
                    .explicit
                    .get_mut(lifetime)
                    .map(|selected| *selected = true);
            },
            // In second pass, we only need to update the first elided lifetime.
            Pass::Second => {
                assert_eq!(&lifetime.ident, self.basename);
                self.state = Pass::Finished;
                lifetime.ident = format_ident!("{}0", self.basename, span = lifetime.span());
                self.elided[self.index].ident = lifetime.ident.clone();
            },
            Pass::Finished => {},
        }
    }

    fn next_lifetime(&mut self, span: Span) -> Lifetime {
        let ident = if self.index == 0 {
            self.basename.clone()
        } else {
            format_ident!("{}{}", self.basename, self.index, span = span)
        };
        let lifetime = Lifetime {
            apostrophe: span,
            ident,
        };
        self.index += 1;
        self.elided.push(lifetime.clone());
        lifetime
    }
}

impl visit_mut::VisitMut for LifetimeCollector<'_> {
    // ignore lifetimes in function pointers, e.g. `fn(&str) -> &str`.
    fn visit_type_bare_fn_mut(&mut self, _: &mut syn::TypeBareFn) {}

    fn visit_fn_arg_mut(&mut self, arg: &mut syn::FnArg) {
        match arg {
            FnArg::Receiver(recv) => self.visit_receiver_mut(recv),
            FnArg::Typed(a) => self.visit_type_mut(&mut a.ty),
        }
    }

    fn visit_receiver_mut(&mut self, recv: &mut syn::Receiver) {
        if let Some((reference, lifetime)) = &mut recv.reference {
            self.collect_opt_lifetime(reference.span, lifetime);
        } else {
            visit_mut::visit_type_mut(self, &mut recv.ty);
        }
    }

    fn visit_type_reference_mut(&mut self, ty: &mut syn::TypeReference) {
        self.collect_opt_lifetime(ty.and_token.span, &mut ty.lifetime);
        if !self.is_finished() {
            self.visit_type_mut(&mut ty.elem);
        }
    }

    fn visit_generic_argument_mut(&mut self, gen: &mut syn::GenericArgument) {
        if let syn::GenericArgument::Lifetime(lifetime) = gen {
            self.collect_lifetime(lifetime);
        } else {
            visit_mut::visit_generic_argument_mut(self, gen);
        }
    }
}

fn default_where_clause(where_clause: &mut Option<syn::WhereClause>) -> &mut syn::WhereClause {
    where_clause.get_or_insert_with(|| syn::WhereClause {
        where_token: <Token![where]>::default(),
        predicates: Punctuated::new(),
    })
}

#[cfg(test)]
#[path = "lifetime_tests.rs"]
mod tests;
