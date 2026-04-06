import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
// import { SolupgPayment } from "../target/types/solupg_payment";
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

describe("solupg-payment", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolupgPayment as Program;
  const payer = provider.wallet as anchor.Wallet;

  let mint: PublicKey;
  let payerTokenAccount: PublicKey;
  let recipientTokenAccount: PublicKey;
  const recipient = Keypair.generate();
  const paymentId = new Uint8Array(16);
  paymentId.set([1, 2, 3, 4]); // simple test ID

  before(async () => {
    // Transfer SOL instead of airdrop (faucet unreliable on Windows)
    const tx = new anchor.web3.Transaction().add(
      SystemProgram.transfer({
        fromPubkey: payer.publicKey,
        toPubkey: recipient.publicKey,
        lamports: 2 * anchor.web3.LAMPORTS_PER_SOL,
      })
    );
    await provider.sendAndConfirm(tx);

    // Create SPL token mint
    mint = await createMint(
      provider.connection,
      payer.payer,
      payer.publicKey,
      null,
      6 // 6 decimals
    );

    // Create token accounts
    payerTokenAccount = await createAccount(
      provider.connection,
      payer.payer,
      mint,
      payer.publicKey
    );

    recipientTokenAccount = await createAccount(
      provider.connection,
      payer.payer,
      mint,
      recipient.publicKey
    );

    // Mint tokens to payer
    await mintTo(
      provider.connection,
      payer.payer,
      mint,
      payerTokenAccount,
      payer.publicKey,
      1_000_000_000 // 1000 tokens
    );
  });

  it("creates a payment", async () => {
    const [paymentStatePda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("payment"),
        payer.publicKey.toBuffer(),
        Buffer.from(paymentId),
      ],
      program.programId
    );

    await program.methods
      .createPayment(
        Array.from(paymentId),
        new anchor.BN(100_000_000), // 100 tokens
        "Test payment"
      )
      .accounts({
        payer: payer.publicKey,
        recipient: recipient.publicKey,
        tokenMint: mint,
        paymentState: paymentStatePda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const state = await program.account.paymentState.fetch(paymentStatePda);
    assert.equal(state.amount.toNumber(), 100_000_000);
    assert.deepEqual(state.status, { pending: {} });
    assert.equal(state.metadata, "Test payment");
  });

  it("executes a payment", async () => {
    const [paymentStatePda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("payment"),
        payer.publicKey.toBuffer(),
        Buffer.from(paymentId),
      ],
      program.programId
    );

    await program.methods
      .executePayment()
      .accounts({
        payer: payer.publicKey,
        paymentState: paymentStatePda,
        tokenMint: mint,
        payerTokenAccount: payerTokenAccount,
        recipientTokenAccount: recipientTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const state = await program.account.paymentState.fetch(paymentStatePda);
    assert.deepEqual(state.status, { executed: {} });

    // Verify token transfer
    const recipientAccount = await getAccount(
      provider.connection,
      recipientTokenAccount
    );
    assert.equal(Number(recipientAccount.amount), 100_000_000);
  });

  it("creates and cancels a payment", async () => {
    const cancelPaymentId = new Uint8Array(16);
    cancelPaymentId.set([5, 6, 7, 8]);

    const [paymentStatePda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("payment"),
        payer.publicKey.toBuffer(),
        Buffer.from(cancelPaymentId),
      ],
      program.programId
    );

    await program.methods
      .createPayment(
        Array.from(cancelPaymentId),
        new anchor.BN(50_000_000),
        "Payment to cancel"
      )
      .accounts({
        payer: payer.publicKey,
        recipient: recipient.publicKey,
        tokenMint: mint,
        paymentState: paymentStatePda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .cancelPayment()
      .accounts({
        payer: payer.publicKey,
        paymentState: paymentStatePda,
      })
      .rpc();

    // Account should be closed after cancel
    try {
      await program.account.paymentState.fetch(paymentStatePda);
      assert.fail("Account should be closed");
    } catch (e) {
      // Expected: account not found
    }
  });
});
