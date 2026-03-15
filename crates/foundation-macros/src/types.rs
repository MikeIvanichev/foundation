use syn::GenericArgument;
use syn::PathArguments;
use syn::Type;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FieldTypeInfo {
    pub(crate) nested: bool,
    pub(crate) optional: bool,
}

impl FieldTypeInfo {
    const fn leaf(optional: bool) -> Self {
        Self {
            nested: false,
            optional,
        }
    }

    const fn nested(optional: bool) -> Self {
        Self {
            nested: true,
            optional,
        }
    }
}

pub(crate) fn classify_type(ty: &Type) -> FieldTypeInfo {
    let Type::Path(type_path) = ty else {
        return FieldTypeInfo::leaf(false);
    };

    let Some(segment) = type_path.path.segments.last() else {
        return FieldTypeInfo::leaf(false);
    };

    let ident = segment.ident.to_string();
    match ident.as_str() {
        "bool" => FieldTypeInfo::leaf(false),
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128"
        | "usize" => FieldTypeInfo::leaf(false),
        "f32" | "f64" => FieldTypeInfo::leaf(false),
        "String" | "str" | "PathBuf" | "Path" | "Url" | "IpAddr" | "SocketAddr" | "Duration" => {
            FieldTypeInfo::leaf(false)
        }
        "Option" => extract_single_type_argument(ty)
            .map(classify_type)
            .map(|inner| FieldTypeInfo {
                nested: inner.nested,
                optional: true,
            })
            .unwrap_or_else(|| FieldTypeInfo::leaf(false)),
        "Vec" | "VecDeque" | "LinkedList" | "BinaryHeap" | "BTreeSet" | "HashSet" | "BTreeMap"
        | "HashMap" | "IndexMap" => FieldTypeInfo::leaf(false),
        _ => {
            if is_scalar_like_name(&ident) {
                FieldTypeInfo::leaf(false)
            } else {
                FieldTypeInfo::nested(false)
            }
        }
    }
}

pub(crate) fn extract_single_type_argument(ty: &Type) -> Option<&Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    let first = args.args.first()?;
    let GenericArgument::Type(inner) = first else {
        return None;
    };
    Some(inner)
}

fn is_scalar_like_name(name: &str) -> bool {
    matches!(
        name,
        "Uri"
            | "HeaderValue"
            | "HeaderName"
            | "NonZeroU8"
            | "NonZeroU16"
            | "NonZeroU32"
            | "NonZeroU64"
            | "NonZeroUsize"
            | "NonZeroI8"
            | "NonZeroI16"
            | "NonZeroI32"
            | "NonZeroI64"
            | "NonZeroIsize"
    )
}
