extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2;
use quote::{format_ident, quote, quote_spanned};
use syn::{
    parse_macro_input, parse_quote, spanned::Spanned, Attribute, Data, DataStruct, DeriveInput,
    ExprArray, Field, Fields, Ident, ItemFn, Lit, Meta, MetaList, MetaNameValue, NestedMeta,
};

/// 用于函数改名的属性过程宏
/// 使用方法，#[rename(path ="func_name")]
#[proc_macro_attribute]
pub fn rename(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attr = proc_macro2::TokenStream::from(attr);
    // dbg!(&attr);
    // 解析为属性，如#[path = "world"]
    let attr: Attribute = parse_quote!(#[#attr]);
    // 解析为syn::Meta
    let meta = attr.parse_meta().unwrap();
    // 找到重定义的名称
    let new_name = match meta {
        Meta::NameValue(MetaNameValue {
            path,
            lit: Lit::Str(s),
            ..
        }) if path.segments[0].ident == "name" => s.value(),
        _ => panic!("only support NameValue attr, like [rename(path = \"fun_1\")]"),
    };
    // 构造函数名Ident
    let new_func_ident = format_ident!("{}", new_name);
    // 解析属性修饰的函数
    let mut fn_item = parse_macro_input!(input as ItemFn);
    // 修改函数名
    fn_item.sig.ident = new_func_ident;
    // 拼接代码
    let expanded = quote! {
        #fn_item
    };
    expanded.into()
}

/// my_vec!([1,2,3])生成let mut _a = vec![]; _a.push(1);_a.push(2);_a.push(3);
#[proc_macro]
pub fn my_vec(input: TokenStream) -> TokenStream {
    // 转化为proc_macro2的Token
    let input = proc_macro2::TokenStream::from(input);
    // 解析为slice表达式，如[1,2,3,4]
    let expr_arr: ExprArray = parse_quote!([#input]);
    // syn::ExprArray中elems是slice表达式中每个元素Expr的迭代器，由此进行代码拼接
    let elem_sets = expr_arr
        .elems
        .iter()
        .map(|e| quote_spanned! {e.span() => _vec.push(#e);});
    // 拼接代码片段，quote插入的变量要求实现ToTokens，包括全部基本类型，syn中的全部rust语法树类型，以及proc_macro2::TokenStream
    // ToTokens的迭代器，通过#(#var)*语法进行重复插入
    let expanded = quote! {
        {
            let mut _vec = std::vec::Vec::new();
            #(#elem_sets)*
            _vec
        }
    };
    // 将proc_macro2::TokenStream转化为proc_macro::TokenStream
    TokenStream::from(expanded)
}

#[proc_macro_derive(Change, attributes(rename))]
pub fn change_derive_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    // 新结构名
    let new_ident = format_ident!("{}Newer", input.ident);
    // 原始结构的字段
    let fields = match input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields_named),
            ..
        }) => fields_named.named,
        _ => panic!("expect struct"),
    };
    // 新结构的字段
    let fields_quote = fields.iter().map(|f| {
        let ty = &f.ty;
        let ident = &f.ident;
        if let Some(name) = get_ident(f) {
            quote_spanned! {f.span() => #name : #ty}
        } else {
            quote_spanned! {f.span() => #ident : #ty }
        }
    });
    let expanded = quote! {
        pub struct #new_ident {
            #(#fields_quote),*
        }
    };
    TokenStream::from(expanded)
}

// 对rename属性修饰的字段进行更名
fn get_ident(filed: &Field) -> Option<Ident> {
    filed.attrs.iter().find_map(|attr| {
        if let Ok(meta) = attr.parse_meta() {
            let new_name = match meta {
                Meta::List(MetaList { path, nested, .. }) if path.segments[0].ident == "rename" => {
                    match &nested[0] {
                        NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                            path,
                            lit: Lit::Str(s),
                            ..
                        })) if path.segments[0].ident == "name" => Some(s.value()),
                        _ => None,
                    }
                }
                _ => None,
            };
            new_name.map(|name| format_ident!("{}", name))
        } else {
            None
        }
    })
}
