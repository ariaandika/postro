Abstraction layer over a sql database operation

# Layer

layered architecture allow for generic operation performed on multiple protocol

## Layer 1, Driver Layer

here we only work with socket and buffer, no protocol specific here.

driver layer provide `Socket`, `ProtocolDecode` and `ProtocolEncode`.

`Socket` is a buffered reader and writer. `Socket` can be written with `ProtocolEncode`
which provide type safe write instead of raw buffer, And `ProtocolDecode` to read
with type safety from the socket.

driver layer also provide common operation like runtime agnostic interface,
tcp or unix stream socket, tls, and more.

## Layer 2, Protocol Layer

any server or client message type must implement `ProtocolDecode` or `ProtocolEncode` respectively.

protocol connection can just hold a `Socket`, then safely send message to server.

# SQLX

## Connection Trait

The [`Executor`] trait represent a type that can execute a query.
For example, it could be a single connection, a pool, or a transaction.

The [`Connection`] trait is a superset of `Executor`, with additional
connection operation, like opening or closing connection.

The [`Acquire`] trait is a type that can retrieve a `Connection`.
This trait is used in connection `Pool`

## Serialization Traits

The [`Database`] trait act like a namespace containing protocol serialization.
Implementor most likely a unit struct.

An `Executor` can execute a query which accept an [`ArgumentBuffer`]

The [`Type`] and [`TypeInfo`] trait represent a rust type that
correspond to database data type.

The [`Encode`] trait represent a type that can be written into [`ArgumentBuffer`].

The [`Decode`] trait represent a type that can be constructed from [`ValueRef`]

[`Arguments`]
[`ArgumentBuffer`]
[`Statement`]
[`QueryResult`]
[`Row`]
[`Column`]
[`Value`]
[`ValueRef`]

## Userspace Traits

[`FromRow`]
