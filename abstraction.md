Abstraction layer over a sql database operation

# Layer

layered architecture allow for generic operation performed on multiple protocol

## Layer 1, Io Layer

here we only work with socket and buffer, no protocol specific here.

this layer provide `Socket` and `BufferedSocket`

this layer also provide common operation like runtime agnostic interface,
tcp or unix stream socket, tls, and more.

## Layer 2, Protocol Layer

io read does not always read the entire message, its protocol specific to know,
buffer wise, is the message is a complete message.

this layer provide `ProtocolStream` which wrap `Socket`, `ProtocolDecode` and `ProtocolEncode`

protocol must have general format for a single message, before decoded more specific message.
in this layer, buffer are read until one message is found, then more specific message type
can decode without performing any io call.



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
