import { cleanSlenderEnvKeys, deploy, releaseInit } from "./pool.sut";
import { SorobanClient } from "./soroban.client";

export async function main() {
  const client = new SorobanClient();

  process.env.NODE_ENV = 'mainnet';

  await cleanSlenderEnvKeys();
  await deploy();
  await releaseInit(client)
}

