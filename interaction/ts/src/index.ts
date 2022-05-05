import * as fs from "fs";
import * as path from "path";
import { TestLoader } from "./loader";

export const testDir = '/home/aries/workplace/elrond-storage-contract/storageOrder/interaction/ts/test';

export async function loadTests(): Promise<TestLoader> {
  const loader = new TestLoader();
  await loader.loadAllTests(testDir);
  return loader;
}
