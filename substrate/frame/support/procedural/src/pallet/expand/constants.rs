// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{deprecation::extract_or_return_allow_attrs, pallet::Def};

struct ConstDef {
	/// Name of the associated type.
	pub ident: syn::Ident,
	/// The type in Get, e.g. `u32` in `type Foo: Get<u32>;`, but `Self` is replaced by `T`
	pub type_: syn::Type,
	/// The doc associated
	pub doc: Vec<syn::Expr>,
	/// default_byte implementation
	pub default_byte_impl: proc_macro2::TokenStream,
	/// Constant name for Metadata (optional)
	pub metadata_name: Option<syn::Ident>,
	/// Deprecation_info:
	pub deprecation_info: proc_macro2::TokenStream,
}

/// Implement the `pallet_constants_metadata` function for the pallet.
pub fn expand_constants(def: &mut Def) -> proc_macro2::TokenStream {
	let frame_support = &def.frame_support;
	let type_impl_gen = &def.type_impl_generics(proc_macro2::Span::call_site());
	let type_use_gen = &def.type_use_generics(proc_macro2::Span::call_site());
	let pallet_ident = &def.pallet_struct.pallet;
	let trait_use_gen = &def.trait_use_generics(proc_macro2::Span::call_site());

	let mut where_clauses = vec![&def.config.where_clause];
	where_clauses.extend(def.extra_constants.iter().map(|d| &d.where_clause));
	let completed_where_clause = super::merge_where_clauses(&where_clauses);

	let mut config_consts = vec![];
	for const_ in def.config.consts_metadata.iter() {
		let ident = &const_.ident;
		let const_type = &const_.type_;
		let deprecation_info = match crate::deprecation::get_deprecation(
			&quote::quote! { #frame_support },
			&const_.attrs,
		) {
			Ok(deprecation) => deprecation,
			Err(e) => return e.into_compile_error(),
		};

		// Extracts #[allow] attributes, necessary so that we don't run into compiler warnings
		let maybe_allow_attrs = extract_or_return_allow_attrs(&const_.attrs);

		config_consts.push(ConstDef {
			ident: const_.ident.clone(),
			type_: const_.type_.clone(),
			doc: const_.doc.clone(),
			default_byte_impl: quote::quote!(
				#(#maybe_allow_attrs)*
				let value = <<T as Config #trait_use_gen>::#ident as
					#frame_support::traits::Get<#const_type>>::get();
				#frame_support::__private::codec::Encode::encode(&value)
			),
			metadata_name: None,
			deprecation_info,
		})
	}

	let mut extra_consts = vec![];
	for const_ in def.extra_constants.iter().flat_map(|d| &d.extra_constants) {
		let ident = &const_.ident;
		let deprecation_info = match crate::deprecation::get_deprecation(
			&quote::quote! { #frame_support },
			&const_.attrs,
		) {
			Ok(deprecation) => deprecation,
			Err(e) => return e.into_compile_error(),
		};
		// Extracts #[allow] attributes, necessary so that we don't run into compiler warnings
		let maybe_allow_attrs = extract_or_return_allow_attrs(&const_.attrs);

		extra_consts.push(ConstDef {
			ident: const_.ident.clone(),
			type_: const_.type_.clone(),
			doc: const_.doc.clone(),
			default_byte_impl: quote::quote!(
				#(#maybe_allow_attrs)*
				let value = <Pallet<#type_use_gen>>::#ident();
				#frame_support::__private::codec::Encode::encode(&value)
			),
			metadata_name: const_.metadata_name.clone(),
			deprecation_info,
		})
	}

	let consts = config_consts.into_iter().chain(extra_consts.into_iter()).map(|const_| {
		let const_type = &const_.type_;
		let ident_str = format!("{}", const_.metadata_name.unwrap_or(const_.ident));

		let no_docs = vec![];
		let doc = if cfg!(feature = "no-metadata-docs") { &no_docs } else { &const_.doc };

		let default_byte_impl = &const_.default_byte_impl;
		let deprecation_info = &const_.deprecation_info;
		quote::quote!({
			#frame_support::__private::metadata_ir::PalletConstantMetadataIR {
				name: #ident_str,
				ty: #frame_support::__private::scale_info::meta_type::<#const_type>(),
				value: { #default_byte_impl },
				docs: #frame_support::__private::vec![ #( #doc ),* ],
				deprecation_info: #deprecation_info
			}
		})
	});

	quote::quote!(
		impl<#type_impl_gen> #pallet_ident<#type_use_gen> #completed_where_clause{

			#[doc(hidden)]
			pub fn pallet_constants_metadata()
				-> #frame_support::__private::Vec<#frame_support::__private::metadata_ir::PalletConstantMetadataIR>
			{
				#frame_support::__private::vec![ #( #consts ),* ]
			}
		}
	)
}
