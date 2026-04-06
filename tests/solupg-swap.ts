import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
// import { SolupgSwap } from "../target/types/solupg_swap";
import {
  Keypair,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import {
  createMint,
  createAccount,
  mintTo,
  getAccount,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { assert } from "chai";

describe("solupg-swap", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolupgSwap as Program;
  const payer = provider.wallet as anchor.Wallet;

  let sourceMint: PublicKey;
  let destinationMint: PublicKey;
  let payerSourceToken: PublicKey;
  let recipientDestToken: PublicKey;
  const recipient = Keypair.generate();
  const swapId = new Uint8Array(16);
  swapId.set([200, 201, 202]);

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

    // Note: In the placeholder implementation, source and dest mints
    // must be different but the transfer is direct (no actual swap).
    // This tests the account validation and flow only.
    sourceMint = await createMint(
      provider.connection,
      payer.payer,
      payer.publicKey,
      null,
      6
    );

    destinationMint = await createMint(
      provider.connection,
      payer.payer,
      payer.publicKey,
      null,
      6
    );

    payerSourceToken = await createAccount(
      provider.connection,
      payer.payer,
      sourceMint,
      payer.publicKey
    );

    // Use sourceMint for placeholder direct transfer (same mint both sides)
    recipientDestToken = await createAccount(
      provider.connection,
      payer.payer,
      sourceMint,
      recipient.publicKey
    );

    await mintTo(
      provider.connection,
      payer.payer,
      sourceMint,
      payerSourceToken,
      payer.publicKey,
      1_000_000_000
    );
  });

  it("executes swap_and_pay (placeholder: direct transfer)", async () => {
    // Note: The placeholder implementation does a direct transfer instead of
    // an actual swap. This test validates the instruction structure.
    // Real Jupiter integration will be tested on devnet.

    // For this placeholder test, we use source_mint as both source and
    // destination_mint would fail (SameToken error), so we pass the
    // destination_mint account but the actual token account uses source_mint.
    // This is acceptable for the scaffold test.

    await program.methods
      .swapAndPay(
        Array.from(swapId),
        new anchor.BN(500_000_000), // 500 tokens
        new anchor.BN(490_000_000), // min 490 tokens (slippage tolerance)
        100 // 1% slippage
      )
      .accounts({
        payer: payer.publicKey,
        recipient: recipient.publicKey,
        sourceMint: sourceMint,
        destinationMint: sourceMint, // Same mint for placeholder direct transfer
        payerSourceToken: payerSourceToken,
        recipientDestinationToken: recipientDestToken,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const recipientAccount = await getAccount(
      provider.connection,
      recipientDestToken
    );
    assert.equal(Number(recipientAccount.amount), 500_000_000);
  });
});
