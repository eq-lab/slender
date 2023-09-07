import { readFileSync, writeFileSync } from "fs";
import { Keypair } from "soroban-client";

const contractsFilename = "../deploy/artifacts/.contracts";

require("dotenv").config({ path: `../deploy/scripts/.${process.env.NODE_ENV}.env` });
require("dotenv").config({ path: contractsFilename });

export const adminKeys = Keypair.fromSecret(process.env.ADMIN_SECRET);
export const treasuryKeys = Keypair.fromSecret(process.env.TREASURY_SECRET);
export const borrower1Keys = Keypair.fromSecret(process.env.BORROWER_1_SECRET);
export const lender1Keys = Keypair.fromSecret(process.env.LENDER_1_SECRET);

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