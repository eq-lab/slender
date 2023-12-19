import { readFileSync, writeFileSync } from "fs";
import { Keypair } from "stellar-sdk";

export const contractsFilename = "../deploy/artifacts/.contracts";

require("dotenv").config({ path: `../deploy/scripts/.${process.env.NODE_ENV}.env` });

export const adminKeys = process.env.ADMIN_SECRET ? Keypair.fromSecret(process.env.ADMIN_SECRET) : Keypair.random();
export const treasuryKeys = process.env.TREASURY_SECRET ? Keypair.fromSecret(process.env.TREASURY_SECRET) : Keypair.random();
export const lender1Keys = process.env.LENDER_1_SECRET ? Keypair.fromSecret(process.env.LENDER_1_SECRET) : Keypair.random();
export const lender2Keys = process.env.LENDER_2_SECRET ? Keypair.fromSecret(process.env.LENDER_2_SECRET) : Keypair.random();
export const lender3Keys = process.env.LENDER_3_SECRET ? Keypair.fromSecret(process.env.LENDER_3_SECRET) : Keypair.random();
export const borrower1Keys = process.env.BORROWER_1_SECRET ? Keypair.fromSecret(process.env.BORROWER_1_SECRET) : Keypair.random();
export const borrower2Keys = process.env.BORROWER_2_SECRET ? Keypair.fromSecret(process.env.BORROWER_2_SECRET) : Keypair.random();
export const liquidator1Keys = process.env.LIQUIDATOR_1_SECRET ? Keypair.fromSecret(process.env.LIQUIDATOR_1_SECRET) : Keypair.random();

export function setEnv(key: string, value: string) {
    const ENV_VARS = readFileSync(contractsFilename, "utf8").split("\n");

    const target = ENV_VARS.indexOf(ENV_VARS.find((line) => {
        return `${line}=`.match(`${key}=`);
    }));

    if (target === -1) {
        ENV_VARS.push(`${key}=${value}`);
    } else {
        ENV_VARS.splice(target, 1, `${key}=${value}`);
    }

    process.env[key] = value;

    writeFileSync(contractsFilename, ENV_VARS.join("\n"));
}
