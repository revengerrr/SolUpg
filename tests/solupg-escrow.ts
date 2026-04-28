import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
// import { SolupgEscrow } from "../target/types/solupg_escrow";
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

describe("solupg-escrow", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolupgEscrow as Program;
  const payer = provider.wallet as anchor.Wallet;

  let mint: PublicKey;
  let payerTokenAccount: PublicKey;
  let recipientTokenAccount: PublicKey;
  const recipient = Keypair.generate();
  const escrowId = new Uint8Array(16);
  escrowId.set([10, 20, 30, 40]);

  before(async () => {
    // Transfer SOL instead of airdrop (faucet unreliable on Windows)
    const tx = new anchor.web3.Transaction().add(
      SystemProgram.transfer({
        fromPubkey: payer.publicKey,
        toPubkey: recipient.publicKey,
        lamports: 2 * anchor.web3.LAMPORTS_PER_SOL,
      })
    );
    const sig = await provider.sendAndConfirm(tx);


    mint = await createMint(
      provider.connection,
      payer.payer,
      payer.publicKey,
      null,
      6
    );

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

    await mintTo(
      provider.connection,
      payer.payer,
      mint,
      payerTokenAccount,
      payer.publicKey,
      1_000_000_000
    );
  });

  it("creates an escrow", async () => {
    const [escrowStatePda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("escrow"),
        payer.publicKey.toBuffer(),
        Buffer.from(escrowId),
      ],
      program.programId
    );

    const [escrowVaultPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("escrow_vault"),
        payer.publicKey.toBuffer(),
        Buffer.from(escrowId),
      ],
      program.programId
    );

    const futureExpiry = Math.floor(Date.now() / 1000) + 3600; // 1 hour from now

    await program.methods
      .createEscrow(
        Array.from(escrowId),
        new anchor.BN(200_000_000), // 200 tokens
        { authorityApproval: {} },
        new anchor.BN(futureExpiry)
      )
      .accounts({
        payer: payer.publicKey,
        recipient: recipient.publicKey,
        tokenMint: mint,
        escrowState: escrowStatePda,
        escrowVault: escrowVaultPda,
        payerTokenAccount: payerTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const state = await program.account.escrowState.fetch(escrowStatePda);
    assert.equal((state as any).amount.toNumber(), 200_000_000);
    assert.deepEqual((state as any).status, { active: {} });
    assert.deepEqual((state as any).releaseCondition, { authorityApproval: {} });

    // Verify tokens are in vault
    const vault = await getAccount(provider.connection, escrowVaultPda);
    assert.equal(Number(vault.amount), 200_000_000);
  });

  it("releases an escrow", async () => {
    const [escrowStatePda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("escrow"),
        payer.publicKey.toBuffer(),
        Buffer.from(escrowId),
      ],
      program.programId
    );

    const [escrowVaultPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("escrow_vault"),
        payer.publicKey.toBuffer(),
        Buffer.from(escrowId),
      ],
      program.programId
    );

    await program.methods
      .releaseEscrow()
      .accounts({
        authority: payer.publicKey,
        escrowState: escrowStatePda,
        tokenMint: mint,
        escrowVault: escrowVaultPda,
        recipientTokenAccount: recipientTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const state = await program.account.escrowState.fetch(escrowStatePda);
    assert.deepEqual(state.status, { released: {} });

    const recipientAccount = await getAccount(
      provider.connection,
      recipientTokenAccount
    );
    assert.equal(Number(recipientAccount.amount), 200_000_000);
  });

  it("creates and cancels an escrow", async () => {
    const cancelEscrowId = new Uint8Array(16);
    cancelEscrowId.set([50, 60, 70, 80]);

    const [escrowStatePda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("escrow"),
        payer.publicKey.toBuffer(),
        Buffer.from(cancelEscrowId),
      ],
      program.programId
    );

    const [escrowVaultPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("escrow_vault"),
        payer.publicKey.toBuffer(),
        Buffer.from(cancelEscrowId),
      ],
      program.programId
    );

    const futureExpiry = Math.floor(Date.now() / 1000) + 3600;

    await program.methods
      .createEscrow(
        Array.from(cancelEscrowId),
        new anchor.BN(100_000_000),
        { authorityApproval: {} },
        new anchor.BN(futureExpiry)
      )
      .accounts({
        payer: payer.publicKey,
        recipient: recipient.publicKey,
        tokenMint: mint,
        escrowState: escrowStatePda,
        escrowVault: escrowVaultPda,
        payerTokenAccount: payerTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .cancelEscrow()
      .accounts({
        payer: payer.publicKey,
        escrowState: escrowStatePda,
        tokenMint: mint,
        escrowVault: escrowVaultPda,
        payerTokenAccount: payerTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const state = await program.account.escrowState.fetch(escrowStatePda);
    assert.deepEqual(state.status, { cancelled: {} });
  });
});
