import { init } from "./pool.sut";
import { SorobanClient } from "./soroban.client";

async function main() {
  const client = new SorobanClient();
  const customXlm = Number(process.argv[2]) === 1;
  await init(client, customXlm);
}

main()
  .catch(console.error)
  .finally(() => process.exit());
