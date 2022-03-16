import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { LuloDex } from "../target/types/lulo_dex";
// Web3
import {
  PublicKey, Keypair, SystemProgram, Transaction, TransactionInstruction, LAMPORTS_PER_SOL,
  SYSVAR_RECENT_BLOCKHASHES_PUBKEY,
  SYSVAR_RENT_PUBKEY
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, NATIVE_MINT, ASSOCIATED_TOKEN_PROGRAM_ID, createMint, createAssociatedTokenAccount, mintToChecked, createSyncNativeInstruction, getAssociatedTokenAddress} from "@solana/spl-token";
import { assert } from "chai";


describe("lulo-dex", () => {
  const provider = anchor.Provider.env();
  anchor.setProvider(anchor.Provider.env());
  const dexProgram = anchor.workspace.LuloDex as Program<LuloDex>;

  // Auths
  const sellerAuth = Keypair.generate();
  const buyerAuth = Keypair.generate();
  const luloAuth = Keypair.generate();

  // Params
  const fee = new anchor.BN(25)
  const feeScalar = new anchor.BN(1000)
  const ask = new anchor.BN(LAMPORTS_PER_SOL);

   // Accounts
   let nftMint = null;
   let sellerNft = null;
   let buyerWsol = null;
   let buyerNft = null;
   let contract = Keypair.generate();

  // PDAs
  let state = null;
  let vault = null;
  let nftVault = null;
  let listing = null;
  let sellerEscrow = null;

  // Bumps
  let sellerEscrowBump = null;
  let stateBump = null;
  let nftVaultBump = null;
  let listingBump = null;
  let vaultBump = null;

  it("Initialize test state", async () => {
    // Airdrop to luloAuth
    const luloAuthAirdrop = await provider.connection.requestAirdrop(luloAuth.publicKey, 100 * LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(luloAuthAirdrop);
    // Airdrop to sellerAuth
    const sellerAuthAirdrop = await provider.connection.requestAirdrop(sellerAuth.publicKey, 100 * LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(sellerAuthAirdrop);
    // Airdrop to buyerAuth
    const buyerAuthAirdrop = await provider.connection.requestAirdrop(buyerAuth.publicKey, 100 * LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(buyerAuthAirdrop);

    nftMint = await createMint(
      provider.connection, // conneciton
      luloAuth, // fee payer
      luloAuth.publicKey, // mint authority
      luloAuth.publicKey, // freeze authority (you can use `null` to disable it. when you disable it, you can't turn it on again)
      0 // decimals
    );

    // Create token account for paying contract
    sellerNft = await createAssociatedTokenAccount(
      provider.connection, // connection
      luloAuth, // fee payer
      nftMint, // mint
      sellerAuth.publicKey // owner,
    );

    // Mint tokens to source
    await mintToChecked(
      provider.connection, // connection
      luloAuth, // fee payer
      nftMint, // mint
      sellerNft, // receiver (sholud be a token account)
      luloAuth, // mint authority
      1, // amount. if your decimals is 8, you mint 10^8 for 1 token.
      0 // decimals
    );

    // Buyer's WSOL account
    buyerWsol = await createAssociatedTokenAccount(
      provider.connection, // connection
      luloAuth, // fee payer
      NATIVE_MINT, // mint
      buyerAuth.publicKey // owner,
    );

    // Fund Buyer WSOL account
    let tx = new Transaction().add(
      // transfer SOL
      SystemProgram.transfer({
        fromPubkey: luloAuth.publicKey,
        toPubkey: buyerWsol,
        lamports: 10 * LAMPORTS_PER_SOL,
      }),
      // sync wrapped SOL balance
      createSyncNativeInstruction(buyerWsol)
    );

    await provider.connection.sendTransaction(tx, [luloAuth]);

    // Ata for buyer
    buyerNft = await getAssociatedTokenAddress(nftMint, buyerAuth.publicKey);

    // State PDA
    [state, stateBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("state")),
      ],
      dexProgram.programId
    );
    // SOL Vault PDA
    [vault, vaultBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("vault")),
        NATIVE_MINT.toBuffer(),
      ],
      dexProgram.programId
    );
    // Listing PDA
    [listing, listingBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("listing")),
        nftMint.toBuffer(),
        sellerAuth.publicKey.toBuffer(),
      ],
      dexProgram.programId
    );
    // NFT vault PDA
    [nftVault, nftVaultBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("vault")),
        nftMint.toBuffer(),
      ],
      dexProgram.programId
    );
    // Seller escrow PDA
    [sellerEscrow, sellerEscrowBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("escrow")),
        sellerAuth.publicKey.toBuffer(),
        NATIVE_MINT.toBuffer(),
      ],
      dexProgram.programId
    );
  });

  it("Initialize dex program", async () => {
    const tx = await dexProgram.rpc.initialize(
      fee,
      feeScalar,
      {
        accounts: {
          signer: luloAuth.publicKey,
          state: state,
          systemProgram: SystemProgram.programId,
        },
        signers: [luloAuth],
      });
      // State data set correctly
      let _state = await dexProgram.account.state.fetch(state);
      assert.ok(_state.admin.equals(luloAuth.publicKey))
      assert.ok(_state.fee.eq(fee))
      assert.ok(_state.feeScalar.eq(feeScalar))
  });

  it("Create vault", async () => {
    const tx = await dexProgram.rpc.createVault(
      {
        accounts: {
          signer: luloAuth.publicKey,
          vault: vault,
          mint: NATIVE_MINT,
          state: state,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY,
        },
        signers: [luloAuth],
      });
      // Vault initialized
      let _vault = await provider.connection.getParsedAccountInfo(vault)
      assert.ok(_vault.value.data['parsed']['info']['mint'] == NATIVE_MINT.toBase58())
      assert.ok(_vault.value.data['parsed']['info']['owner'] == vault.toBase58())
  });

  it("List", async () => {
    const tx = await dexProgram.rpc.list(
      ask,
      {
        accounts: {
          signer: sellerAuth.publicKey,
          listing: listing,
          sellerNft: sellerNft,
          nftVault: nftVault,
          nftMint: nftMint,
          sellerEscrow: sellerEscrow,
          contract: Keypair.generate().publicKey,
          askMint: NATIVE_MINT,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY,
        },
        signers: [sellerAuth],
      });
      // Listing created
      let _listing = await dexProgram.account.listing.fetch(listing)
      assert.ok(_listing.seller.equals(sellerAuth.publicKey))
      assert.ok(_listing.ask.eq(ask))
      assert.ok(_listing.mint.equals(nftMint))
      assert.ok(_listing.askMint.equals(NATIVE_MINT))
    });

    it("Buy", async () => {
      const tx = await dexProgram.rpc.buy(
        {
          accounts: {
            signer: buyerAuth.publicKey,
            seller: sellerAuth.publicKey,
            source: buyerWsol,
            listing: listing,
            sellerEscrow: sellerEscrow,
            destination: buyerNft,
            nftVault: nftVault,
            nftMint: nftMint,
            systemProgram: SystemProgram.programId,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            tokenProgram: TOKEN_PROGRAM_ID,
            rent: SYSVAR_RENT_PUBKEY,
          },
          signers: [buyerAuth],
        });
        // NFT with buyer
        let _balance = await provider.connection.getTokenAccountBalance(buyerNft)
        assert.ok(_balance.value.amount == '1')
      });
});
