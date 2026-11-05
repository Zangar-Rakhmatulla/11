import {
  Connection, Keypair, PublicKey,
  Transaction, TransactionInstruction, sendAndConfirmTransaction,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID, getAccount,
} from "@solana/spl-token";
import fs from "fs";
import os from "os";

const PROGRAM_ID = new PublicKey("AUov6zyDFS4R86PtEMxeikWXfXAc5j6pHxXtZcPkjBRv");
const VAULT = new PublicKey("HyxdbjCrUaU6JSdNKVvNWNJmgr7J7FCkQCgb8Hut7mjK");
const SHARE_MINT = new PublicKey("HVt9UCkQ7aVgujdvgW5rs4x32Ckr9xEeZMqjKSNLQp9W");
const VAULT_TOKEN = new PublicKey("EUTxbHc2DhM9h79V8B6seSYLkkoBNmWR1YsqWzfaFiiV");
const POSITION = new PublicKey("CdeERQm9Z2VfTFMsqMHDwEXBsnGdmKGZDV2faR8Bijqn");

const secret = JSON.parse(fs.readFileSync(os.homedir() + "/.config/solana/id.json", "utf8"));
const user = Keypair.fromSecretKey(Uint8Array.from(secret));
const conn = new Connection("https://api.devnet.solana.com", "confirmed");

const sharesUi = Number(process.argv[2] ?? "5");
const sharesToBurn = BigInt(Math.round(sharesUi * 1_000_000));

const tokenAccounts = await conn.getParsedTokenAccountsByOwner(user.publicKey, { mint: new PublicKey("5pTPEE31RcmUqmFJw57k17rGx6mBTvbYJUqRGZ4erMKG") });
const userTokenAddress = new PublicKey(tokenAccounts.value[0].pubkey);

const shareAccounts = await conn.getParsedTokenAccountsByOwner(user.publicKey, { mint: SHARE_MINT });
const userShareAddress = new PublicKey(shareAccounts.value[0].pubkey);

const idl = JSON.parse(fs.readFileSync("../target/idl/liquidity_vault.json", "utf8"));
const ix = idl.instructions.find((i: any) => i.name === "withdraw");
const data = Buffer.alloc(16);
Buffer.from(ix.discriminator).copy(data, 0);
data.writeBigUInt64LE(sharesToBurn, 8);

const instruction = new TransactionInstruction({
  programId: PROGRAM_ID,
  keys: [
    { pubkey: user.publicKey, isSigner: true, isWritable: true },
    { pubkey: VAULT, isSigner: false, isWritable: true },
    { pubkey: SHARE_MINT, isSigner: false, isWritable: true },
    { pubkey: VAULT_TOKEN, isSigner: false, isWritable: true },
    { pubkey: userTokenAddress, isSigner: false, isWritable: true },
    { pubkey: userShareAddress, isSigner: false, isWritable: true },
    { pubkey: POSITION, isSigner: false, isWritable: true },
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
  ],
  data,
});

const sig = await sendAndConfirmTransaction(conn, new Transaction().add(instruction), [user]);

const bal = async (a: PublicKey) => Number((await getAccount(conn, a)).amount) / 1e6;
console.log("Withdrawn shares:", sharesUi);
console.log("User tokens:     ", await bal(userTokenAddress));
console.log("User share-tokens:", await bal(userShareAddress));
console.log("Vault tokens:     ", await bal(VAULT_TOKEN));
console.log("TX:", `https://explorer.solana.com/tx/${sig}?cluster=devnet`);
