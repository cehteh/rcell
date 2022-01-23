* RCell, because ~~all~~most its methods start with a "r"

A Wrapper for Arc that can be `Arc<T>` or `Weak<T>` allowing one to retain values selectively.

To be used when one has to store a reference to some data but if this reference needs to keep
it alive or not is to be determined at runtime.
