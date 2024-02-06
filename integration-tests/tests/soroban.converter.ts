import { Address, xdr } from 'stellar-sdk';
import { Buffer } from "node:buffer";
import { bufToBigint } from 'bigint-conversion';

type ElementType<T> = T extends Array<infer U> ? U : never;
type KeyType<T> = T extends Map<infer K, any> ? K : never;
type ValueType<T> = T extends Map<any, infer V> ? V : never;

export function convertToScvAddress(value: string): xdr.ScVal {
    let addrObj = Address.fromString(value);
    return addrObj.toScVal();
}

export function convertToScvBool(value: boolean): xdr.ScVal {
    return xdr.ScVal.scvBool(value);
}

export function convertToScvMap(value: object): xdr.ScVal {
    const map = Object
        .keys(value)
        .map(k => new xdr.ScMapEntry({
            key: xdr.ScVal.scvSymbol(k),
            val: value[k]
        }));

    return xdr.ScVal.scvMap(map);
}

export function convertToScvEnum(key: string, values: xdr.ScVal[] = []): xdr.ScVal {
    const symbol = xdr.ScVal.scvSymbol(key);
    return xdr.ScVal.scvVec([symbol, ...values]);
}

export function convertToScvVec(value: xdr.ScVal[]): xdr.ScVal {
    return xdr.ScVal.scvVec(value);
}

export function convertToScvI128(value: bigint): xdr.ScVal {
    return xdr.ScVal.scvI128(new xdr.Int128Parts({
        lo: xdr.Uint64.fromString((value & BigInt(0xFFFFFFFFFFFFFFFFn)).toString()),
        hi: xdr.Int64.fromString(((value >> BigInt(64)) & BigInt(0xFFFFFFFFFFFFFFFFn)).toString()),
    }))
}

export function convertToScvU128(value: bigint): xdr.ScVal {
    return xdr.ScVal.scvU128(new xdr.UInt128Parts({
        lo: xdr.Uint64.fromString((value & BigInt(0xFFFFFFFFFFFFFFFFn)).toString()),
        hi: xdr.Int64.fromString(((value >> BigInt(64)) & BigInt(0xFFFFFFFFFFFFFFFFn)).toString()),
    }))
}

export function convertToScvU32(value: number): xdr.ScVal {
    return xdr.ScVal.scvU32(value);
}

export function convertToScvU64(value: number): xdr.ScVal {
    return xdr.ScVal.scvU64(new xdr.Uint64(value));
}

export function convertToScvString(value: string): xdr.ScVal {
    return xdr.ScVal.scvString(value);
}

export function convertToScvBytes(value: string, encoding: BufferEncoding): xdr.ScVal {
    const bytes = Buffer.from(value, encoding);
    return xdr.ScVal.scvBytes(bytes);
}

export function parseMetaXdrToJs<T>(meta: xdr.TransactionMeta): T {
    const value = meta.v3()
        .sorobanMeta()
        .returnValue();

    return parseScvToJs(value);
}

export function parseScvToJs<T>(val: xdr.ScVal): T {
    switch (val?.switch()) {
        case xdr.ScValType.scvBool(): {
            return val.b() as unknown as T;
        }
        case xdr.ScValType.scvVoid():
        case undefined: {
            return undefined;
        }
        case xdr.ScValType.scvU32(): {
            return val.u32() as unknown as T;
        }
        case xdr.ScValType.scvI32(): {
            return val.i32() as unknown as T;
        }
        case xdr.ScValType.scvU64():
        case xdr.ScValType.scvI64():
        case xdr.ScValType.scvU128():
        case xdr.ScValType.scvI128():
        case xdr.ScValType.scvU256():
        case xdr.ScValType.scvI256(): {
            return parseScvToBigInt(val) as unknown as T;
        }
        case xdr.ScValType.scvAddress(): {
            return Address.fromScVal(val).toString() as unknown as T;
        }
        case xdr.ScValType.scvString(): {
            return val.str().toString() as unknown as T;
        }
        case xdr.ScValType.scvSymbol(): {
            return val.sym().toString() as unknown as T;
        }
        case xdr.ScValType.scvBytes(): {
            return val.bytes() as unknown as T;
        }
        case xdr.ScValType.scvVec(): {
            type Element = ElementType<T>;
            return val.vec().map(v => parseScvToJs<Element>(v)) as unknown as T;
        }
        case xdr.ScValType.scvMap(): {
            type Key = KeyType<T>;
            type Value = ValueType<T>;
            let res: any = {};
            val.map().forEach((e) => {
                let key = parseScvToJs<Key>(e.key());
                let value;
                let v: xdr.ScVal = e.val();

                switch (v?.switch()) {
                    case xdr.ScValType.scvMap(): {
                        let inner_map = new Map() as Map<any, any>;
                        v.map().forEach((e) => {
                            let key = parseScvToJs<Key>(e.key());
                            let value = parseScvToJs<Value>(e.val());
                            inner_map.set(key, value);
                        });
                        value = inner_map;
                        break;
                    }
                    default: {
                        value = parseScvToJs<Value>(e.val());
                    }
                }

                res[key as Key] = value as Value;
            });
            return res as unknown as T
        }
        case xdr.ScValType.scvLedgerKeyNonce():
            return val.nonceKey() as unknown as T;
        case xdr.ScValType.scvTimepoint():
            return val.timepoint() as unknown as T;
        case xdr.ScValType.scvDuration():
            return val.duration() as unknown as T;

        default: {
            throw new Error(`type not implemented yet: ${val?.switch().name}`);
        }
    };
}

function parseScvToBigInt(scval: xdr.ScVal | undefined): BigInt {
    switch (scval?.switch()) {
        case undefined: {
            return undefined;
        }
        case xdr.ScValType.scvU64(): {
            const { high, low } = scval.u64();
            return bufToBigint(new Uint32Array([high, low]));
        }
        case xdr.ScValType.scvI64(): {
            const { high, low } = scval.i64();
            return bufToBigint(new Int32Array([high, low]));
        }
        case xdr.ScValType.scvU128(): {
            const parts = scval.u128();
            const a = parts.hi();
            const b = parts.lo();
            return bufToBigint(new Uint32Array([a.high, a.low, b.high, b.low]));
        }
        case xdr.ScValType.scvI128(): {
            const parts = scval.i128();
            return BigInt(parts.lo().toString()) | (BigInt(parts.hi().toString()) << BigInt(64));
        }
        default: {
            throw new Error(`Invalid type for scvalToBigInt: ${scval?.switch().name}`);
        }
    };
}
