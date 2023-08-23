import { init } from "./pool.sut";
import { SorobanClient } from "./soroban.client";

async function main() {
  const client = new SorobanClient();
  await init(client);
}

main()
  .catch(console.error)
  .finally(() => process.exit());
