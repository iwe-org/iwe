use liwe::query::block::{BlockPredicate, IntoBlockPredicate};

pub fn any() -> BlockPredicate {
    BlockPredicate::empty()
}

pub fn text(text: &str) -> BlockPredicate {
    any().text(text)
}

pub fn text_eq(text: &str) -> BlockPredicate {
    any().text_eq(text)
}

pub fn matches(pattern: &str) -> BlockPredicate {
    any().matches(pattern)
}

pub fn within(scope: BlockPredicate) -> BlockPredicate {
    any().within(scope)
}

pub fn within_section(title: &str) -> BlockPredicate {
    any().within_section(title)
}

pub fn contains(pred: BlockPredicate) -> BlockPredicate {
    any().contains(pred)
}

pub fn section(root: impl IntoBlockPredicate) -> BlockPredicate {
    any().section(root)
}

pub fn sections() -> BlockPredicate {
    section(any())
}

pub fn quote(inner: BlockPredicate) -> BlockPredicate {
    any().quote(inner)
}

pub fn quotes() -> BlockPredicate {
    quote(any())
}

pub fn list(inner: BlockPredicate) -> BlockPredicate {
    any().list(inner)
}

pub fn lists() -> BlockPredicate {
    list(any())
}

pub fn header(arg: impl IntoBlockPredicate) -> BlockPredicate {
    any().header(arg)
}

pub fn headers() -> BlockPredicate {
    header(any())
}

pub fn paragraph(arg: impl IntoBlockPredicate) -> BlockPredicate {
    any().paragraph(arg)
}

pub fn paragraphs() -> BlockPredicate {
    paragraph(any())
}

pub fn item(arg: impl IntoBlockPredicate) -> BlockPredicate {
    any().item(arg)
}

pub fn items() -> BlockPredicate {
    item(any())
}

pub fn code(arg: impl IntoBlockPredicate) -> BlockPredicate {
    any().code(arg)
}

pub fn code_blocks() -> BlockPredicate {
    code(any())
}

pub fn table(arg: impl IntoBlockPredicate) -> BlockPredicate {
    any().table(arg)
}

pub fn tables() -> BlockPredicate {
    table(any())
}

pub fn reference(inner: BlockPredicate) -> BlockPredicate {
    any().reference(inner)
}

pub fn refs() -> BlockPredicate {
    reference(any())
}

pub fn hr(inner: BlockPredicate) -> BlockPredicate {
    any().hr(inner)
}

pub fn rules() -> BlockPredicate {
    hr(any())
}

pub fn references(key: &str) -> BlockPredicate {
    any().references(key)
}

pub fn and(preds: Vec<BlockPredicate>) -> BlockPredicate {
    any().and(preds)
}

pub fn or(preds: Vec<BlockPredicate>) -> BlockPredicate {
    any().or(preds)
}

pub fn nor(preds: Vec<BlockPredicate>) -> BlockPredicate {
    any().nor(preds)
}
