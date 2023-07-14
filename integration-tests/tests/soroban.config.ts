import { Keypair } from "soroban-client";

require("dotenv").config({ path: `.${process.env.NODE_ENV}.env` });
require("dotenv").config({ path: ".contracts" });

export const tokenKeys = Keypair.fromSecret(process.env.TOKEN_SECRET);
export const poolKeys = Keypair.fromSecret(process.env.POOL_SECRET);
export const userKeys = Keypair.fromSecret(process.env.USER_SECRET);
