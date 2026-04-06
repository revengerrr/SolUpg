import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
// import { SolupgSplitter } from "../target/types/solupg_splitter";
import {
  Keypair,
  SystemProgram,
  PublicKey,
} from "@solana/web3.js";
import {
  createMint,
  createAccount,
  mintTo,
  getAccount,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { assert } from "chai";

describe("solupg-splitter", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolupgSplitter as Program;
  const authority = provider.wallet as anchor.Wallet;

  let mint: PublicKey;
  let senderTokenAccount: PublicKey;
  let merchantTokenAccount: PublicKey;
  let platformTokenAccount: PublicKey;
  let referrerTokenAccount: PublicKey;

  const merchant = Keypair.generate();
  const platform = Keypair.generate();
  const referrer = Keypair.generate();
  const configId = new Uint8Array(16);
  configId.set([100, 101, 102]);

  before(async () => {
    // Transfer SOL instead of airdrop (faucet unreliable on Windows)
    for (const kp of [merchant, platform, referrer]) {
      const tx = new anchor.web3.Transaction().add(
        SystemProgram.transfer({
          fromPubkey: authority.publicKey,
          toPubkey: kp.publicKey,
          lamports: anchor.web3.LAMPORTS_PER_SOL,
        })
      );
      await provider.sendAndConfirm(tx);
    }

    mint = await createMint(
      provider.connection,
      authority.payer,
      authority.publicKey,
      null,
      6
    );

    senderTokenAccount = await createAccount(
      provider.connection,
      authority.payer,
      mint,
      authority.publicKey
    );

    merchantTokenAccount = await createAccount(
      provider.connection,
      authority.payer,
      mint,
      merchant.publicKey
    );

    platformTokenAccount = await createAccount(
      provider.connection,
      authority.payer,
      mint,
      platform.publicKey
    );

    referrerTokenAccount = await createAccount(
      provider.connection,
      authority.payer,
      mint,
      referrer.publicKey
    );

    await mintTo(
      provider.connection,
      authority.payer,
      mint,
      senderTokenAccount,
      authority.publicKey,
      1_000_000_000
    );
  });

  it("creates a split config", async () => {
    const [splitConfigPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("split_config"),
        authority.publicKey.toBuffer(),
        Buffer.from(configId),
      ],
      program.programId
    );

    await program.methods
      .createSplitConfig(
        Array.from(configId),
        [merchant.publicKey, platform.publicKey, referrer.publicKey],
        [9700, 200, 100] // 97%, 2%, 1%
      )
      .accounts({
        authority: authority.publicKey,
        tokenMint: mint,
        splitConfig: splitConfigPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const config = await program.account.splitConfig.fetch(splitConfigPda);
    assert.equal(config.recipients.length, 3);
    assert.deepEqual(config.ratios, [9700, 200, 100]);
  });

  it("executes a split", async () => {
    const [splitConfigPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("split_config"),
        authority.publicKey.toBuffer(),
        Buffer.from(configId),
      ],
      program.programId
    );

    const amount = 1_000_000; // 1 token

    await program.methods
      .executeSplit(new anchor.BN(amount))
      .accounts({
        sender: authority.publicKey,
        splitConfig: splitConfigPda,
        senderTokenAccount: senderTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .remainingAccounts([
        { pubkey: merchantTokenAccount, isSigner: false, isWritable: true },
        { pubkey: platformTokenAccount, isSigner: false, isWritable: true },
        { pubkey: referrerTokenAccount, isSigner: false, isWritable: true },
      ])
      .rpc();

    const merchantAcct = await getAccount(provider.connection, merchantTokenAccount);
    const platformAcct = await getAccount(provider.connection, platformTokenAccount);
    const referrerAcct = await getAccount(provider.connection, referrerTokenAccount);

    // 97% of 1_000_000 = 970_000
    assert.equal(Number(merchantAcct.amount), 970_000);
    // 2% of 1_000_000 = 20_000
    assert.equal(Number(platformAcct.amount), 20_000);
    // Remainder: 1_000_000 - 970_000 - 20_000 = 10_000
    assert.equal(Number(referrerAcct.amount), 10_000);
  });

  it("updates a split config", async () => {
    const [splitConfigPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("split_config"),
        authority.publicKey.toBuffer(),
        Buffer.from(configId),
      ],
      program.programId
    );

    await program.methods
      .updateSplitConfig(
        [merchant.publicKey, platform.publicKey],
        [9500, 500] // 95%, 5%
      )
      .accounts({
        authority: authority.publicKey,
        splitConfig: splitConfigPda,
      })
      .rpc();

    const config = await program.account.splitConfig.fetch(splitConfigPda);
    assert.equal(config.recipients.length, 2);
    assert.deepEqual(config.ratios, [9500, 500]);
  });
});
