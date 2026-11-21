import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import fs from "fs";
import os from "os";

const PROGRAM_ID = new PublicKey("AUov6zyDFS4R86PtEMxeikWXfXAc5j6pHxXtZcPkjBRv");
const TOKEN_MINT = new PublicKey("5pTPEE31RcmUqmFJw57k17rGx6mBTvbYJUqRGZ4erMKG");

function writeU64LE(val: bigint): Buffer {
  const buf = Buffer.alloc(8);
  buf.writeBigUInt64LE(val, 0);
  return buf;
}

function encodeVecU8(bytes: Buffer): Buffer {
  const lenBuf = Buffer.alloc(4);
  lenBuf.writeUInt32LE(bytes.length, 0);
  return Buffer.concat([lenBuf, bytes]);
}

async function main() {
  const secret = JSON.parse(
    fs.readFileSync(os.homedir() + "/.config/solana/id.json", "utf8")
  );
  const admin = Keypair.fromSecretKey(Uint8Array.from(secret));
  const conn = new Connection("https://api.devnet.solana.com", "confirmed");

  const vaultTokenOutEnv = process.env.VAULT_TOKEN_OUT;
  const clmmProgramEnv = process.env.CLMM_PROGRAM_ID;
  if (!vaultTokenOutEnv || !clmmProgramEnv) {
    console.log("Swap client ready. Set VAULT_TOKEN_OUT and CLMM_PROGRAM_ID to send tx.");
    console.log("Program ID:", PROGRAM_ID.toBase58());
    console.log("Admin:     ", admin.publicKey.toBase58());
    return;
  }

  const vaultTokenOut = new PublicKey(vaultTokenOutEnv);
  const clmmProgram = new PublicKey(clmmProgramEnv);

  const amountIn = BigInt(process.env.AMOUNT_IN || "1000000");
  const minAmountOut = BigInt(process.env.MIN_AMOUNT_OUT || "0");
  const swapIxData = Buffer.from(process.env.SWAP_IX_DATA || "", "hex");

  const idl = JSON.parse(
    fs.readFileSync("target/idl/liquidity_vault.json", "utf8")
  );
  const ixDef = idl.instructions.find((i: any) => i.name === "swap");
  if (!ixDef || !ixDef.discriminator) {
    throw new Error("swap instruction or discriminator not found in IDL");
  }

  const [vault] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), TOKEN_MINT.toBuffer()],
    PROGRAM_ID
  );
  const [vaultTokenIn] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault_token"), vault.toBuffer()],
    PROGRAM_ID
  );

  const data = Buffer.concat([
    Buffer.from(ixDef.discriminator),
    writeU64LE(amountIn),
    writeU64LE(minAmountOut),
    encodeVecU8(swapIxData),
  ]);

  const instruction = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: admin.publicKey, isSigner: true, isWritable: true },
      { pubkey: vault, isSigner: false, isWritable: true },
      { pubkey: vaultTokenIn, isSigner: false, isWritable: true },
      { pubkey: vaultTokenOut, isSigner: false, isWritable: true },
      { pubkey: clmmProgram, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data,
  });

  const sig = await sendAndConfirmTransaction(
    conn,
    new Transaction().add(instruction),
    [admin]
  );
  console.log("SWAP TX:", `https://explorer.solana.com/tx/${sig}?cluster=devnet`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
