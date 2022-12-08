* RCell, because ~~all~~ most its methods start with a "r"

A Wrapper for reference counted cell that can be `Strong<T>`, `Weak<T>` or `Empty` allowing one to
retain values selectively.

To be used when one has to store a reference to some data but if this reference needs to keep
it alive or not is to be determined at runtime.

The feature **sync** which is enabled by default selects `std::sync::Arc<T>` and
`std::sync::Weak<T>` as `rcell::Strong<T>` and `rcell::Weak<T>`. When the **sync** feature is
disabled then the non sync `std::rc::Rc<T>` and `std::rc::Weak<T>` are selected as
`rcell::Strong<T>` and `rcell::Weak<T>`.
