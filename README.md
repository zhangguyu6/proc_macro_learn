# proc_macro是什么？
proc_macro是Rust标准库提供的，支持用户自定义宏的lib，包括函数过程宏#[proc_macro]，属性过程宏#[proc_macro_attribute]和推导过程宏#[proc_macro_derive]。三种宏的使用大同小异，都是接受代码，即TokenStream作为输入，调用syn方法转化为syn的语法树，从中找到相关信息，再使用quote重新拼接TokenStream。

# 相关生态
如上文提到的，proc_macro是std自带的，使用时,需要在`Carog.toml`指定：
```toml
[lib]
proc-macro = true
```
syn用于将代码解析成语法树，quote用于代码拼接, proc-macro2是proc-macro的包装，quote和syn使用其进行语法处理：
```toml
[dependencies]
proc-macro2 = "1.0"
quote = "1.0"
syn = { version = "1.0", features = ["full", "extra-traits"] }
```
Trybuild用于对宏进行单元测试，它在测试时调用rustc对目标文件进行编译，检查编译是否通过：
```toml
[dev-dependencies]
trybuild = "1.0"
```
# 函数过程宏#[proc_macro]
## 实现一个类似vec!的函数过程宏my_vec!，
```rust
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
    // 将proc_macro2::TokenStream转化proc_macro::TokenStream
    TokenStream::from(expanded)
}
```
## 函数过程宏的使用
**正常使用** 

正常使用函数过程宏，直接像函数一直进行调用，入参会作为input传给声明的过程宏，如下面代码传给过程宏my_vec!的TokenSteam为`1, 1`
```rust
fn main() {
    let a1: Vec<u8> = my_vec!(1, 1);
    println!("{:?}",a1);
}
```
输出`[1,1]` 

**错误使用**  
错误使用会在编译时报错
```rust
fn main() {
    let a1:Vec<u8> = my_vec!(1,'c');
}
```
编译错误，提示
```rust
let a1:Vec<u8> = my_vec!(1,'c');
                            ^^^ expected integer, found `char`
```
还有一点要注意，函数过程宏目前直接在expr或者statement中使用，需要开启`#![feature(proc_macro_hygiene)]`特性

# 属性过程宏#[proc_macro_attribute]
接受属性和属性修饰的代码片段，生成代码的过程宏
## 实现一个改名的属性过程宏
``` rust
/// 用于函数改名的属性过程宏
/// 使用方法，#[rename(path ="func_name")]
#[proc_macro_attribute]
pub fn rename(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attr = proc_macro2::TokenStream::from(attr);
    // 解析为属性，如#[rename(path = "world")]
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
```
## 属性过程宏的正常使用
属性作为第一个入参，属性修饰的Item作为第二个入参，传入之前声明的rename过程宏
```rust
#[rename(name = "world")]
fn hello() {
    println!("hello world!");
}

fn main() {
    world();
}
```
# 推导过程宏#[proc_macro_derive]
接受属性和结构，生成代码的过程宏，与其他宏不同，原声明的结构是一直存在的，宏产生的代码不会覆盖源代码
## 生成一个对结构进行改名的推导过程宏
```rust
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
#[proc_macro_derive(Change, attributes(rename))]
pub fn change_derive_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let new_ident = format_ident!("{}Newer", input.ident);
    let fields = match input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields_named),
            ..
        }) => fields_named.named,
        _ => panic!("expect struct"),
    };
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
```
## 推导宏的正常使用
```rust
#[derive(Change)]
struct A {
    #[rename(name = "b")]
    a: u8,
    c: u8,
}

fn main() {
    let a1: ANewer = ANewer { b: 1, c: 2 };
}
```