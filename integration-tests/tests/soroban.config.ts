import { Keypair } from "soroban-client";

require("dotenv").config({ path: `.${process.env.NODE_ENV}.env` });
require("dotenv").config({ path: ".contracts" });

export const adminKeys = Keypair.fromSecret(process.env.ADMIN_SECRET);
export const treasuryKeys = Keypair.fromSecret(process.env.TREASURY_SECRET);
export const borrower1Keys = Keypair.fromSecret(process.env.BORROWER_1_SECRET);
export const lender1Keys = Keypair.fromSecret(process.env.LENDER_1_SECRET);
