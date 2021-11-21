import * as anchor from '@project-serum/anchor';
import { Program } from '@project-serum/anchor';
import { GmootStaking } from '../target/types/gmoot_staking';
import * as splToken from '@solana/spl-token';
import { expect } from 'chai';

describe('gmoot-staking', () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());
  const provider = anchor.getProvider();

  const gmootStakingProgram = anchor.workspace.GmootStaking as Program<GmootStaking>;
  const systemProgram = anchor.web3.SystemProgram.programId;
  const rentSysvar = anchor.web3.SYSVAR_RENT_PUBKEY;
  const clockSysvar = anchor.web3.SYSVAR_CLOCK_PUBKEY;

  const mintNFT = async (connection: anchor.web3.Connection, owner: anchor.web3.Signer): Promise<[splToken.Token, anchor.web3.PublicKey]> => {
      console.log("creating NFT mint");
      const nftMint = await splToken.Token.createMint(
        provider.connection,
        owner,
        owner.publicKey,
        null,
        0,
        splToken.TOKEN_PROGRAM_ID
      );
      const nftTokenAccount = await nftMint.createAssociatedTokenAccount(owner.publicKey);
      console.log("minting nft");
      await nftMint.mintTo(nftTokenAccount, owner, [], 1);
      console.log("removing mint authority");
      await nftMint.setAuthority(nftMint.publicKey, null, 'MintTokens', owner, []);
    return [nftMint, nftTokenAccount];
  }

  const createPDAAssociatedTokenAccount = async (
    connection: anchor.web3.Connection,
    mint: anchor.web3.PublicKey,
    owner: anchor.web3.PublicKey,
    payer: anchor.web3.Signer): Promise<anchor.web3.PublicKey> => {
      console.log("finding PDA AssociatedTokenAddress");
      const tokenAccountAddress = await splToken.Token.getAssociatedTokenAddress(
        splToken.ASSOCIATED_TOKEN_PROGRAM_ID,
        splToken.TOKEN_PROGRAM_ID,
        mint,
        owner,
        true,
      );

      console.log('creating PDA AssociatedTokenAccount');
      let tx = new anchor.web3.Transaction();
      tx.add(splToken.Token.createAssociatedTokenAccountInstruction(
        splToken.ASSOCIATED_TOKEN_PROGRAM_ID,
        splToken.TOKEN_PROGRAM_ID,
        mint,
        tokenAccountAddress,
        owner,
        payer.publicKey
      ))

      let txId = await connection.sendTransaction(tx, [payer]);
      await connection.confirmTransaction(txId, 'confirmed');
      return tokenAccountAddress;
  };

  describe('end to end test', async () => {
    const owner = anchor.web3.Keypair.generate();
    const [rewarder, rewarderBump] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from("gmoot"), Buffer.from("rewarder")], gmootStakingProgram.programId);
    const [rewardAuthority, rewardAuthorityBump] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from("gmoot"), Buffer.from("rewarder"), rewarder.toBuffer()], gmootStakingProgram.programId);
    const [stakeAccount, stakeAccountBump] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from("gmoot"), Buffer.from("stake_account"), owner.publicKey.toBuffer()], gmootStakingProgram.programId);
    const rewardRate = 10;
    let rewardMint = null;
    let rewardTokenAccount = null;
    let nftMint = null;
    let nftTokenAccount = null;
    let nftVault = null;


    before(async () => {
      console.log('airdropping to owner');
      //airdrop tokens
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(owner.publicKey, 1000000000),
        "confirmed"
      );

      console.log("creating reward mint");
      rewardMint = await splToken.Token.createMint(
        provider.connection,
        owner, //payer
        rewardAuthority, //mint authority
        null, //freeze authority
        3, //deicmals
        splToken.TOKEN_PROGRAM_ID
      );

      console.log("creating reward token account")
      rewardTokenAccount = await rewardMint.createAssociatedTokenAccount(owner.publicKey);

      console.log("minting NFT");
      [nftMint, nftTokenAccount] = await mintNFT(provider.connection, owner);

      console.log("creating NFT Vault")
      nftVault = await createPDAAssociatedTokenAccount(provider.connection, nftMint.publicKey, stakeAccount, owner);
      
    });

    it('initializes a rewarder', async () => {
      await gmootStakingProgram.rpc.initializeRewarder(rewarderBump, rewardAuthorityBump, new anchor.BN(rewardRate), {
        accounts: {
          rewarder: rewarder,
          authority: owner.publicKey,
          rewardAuthority: rewardAuthority,
          rewardMint: rewardMint.publicKey,
          systemProgram,
          rent: rentSysvar
        },
        signers: [owner]
      });
    });

    it('initialized a stake account', async () => {
      await gmootStakingProgram.rpc.initializeStakeAccount(stakeAccountBump, {
        accounts: {
          owner: owner.publicKey,
          stakeAccount,
          rewarder,
          systemProgram,
          rent: rentSysvar,
        },
        signers: [owner]
      });
    });

    it('stakes an NFT', async () => {
      await gmootStakingProgram.rpc.stakeGmoot({
        accounts: {
          owner: owner.publicKey,
          rewarder,
          rewardAuthority,
          stakeAccount,
          rewardMint: rewardMint.publicKey,
          rewardTokenAccount,
          nftMint: nftMint.publicKey,
          nftTokenAccount,
          nftVault,
          tokenProgram: splToken.TOKEN_PROGRAM_ID,
          systemProgram,
          rent: rentSysvar,
          clock: clockSysvar,
        },
        signers: [owner]
      });

      let nftAccount = await nftMint.getAccountInfo(nftTokenAccount);
      expect(nftAccount.amount.toNumber()).to.equal(0);
      let nftVaultAccount = await nftMint.getAccountInfo(nftVault);
      expect(nftVaultAccount.amount.toNumber()).to.equal(1);
    });

    it('claims pending rewards', async () => {
      const seconds = 2;
      //wait to allow rewards to accumulate
      await sleep(provider.connection, seconds);

      await gmootStakingProgram.rpc.claim({
        accounts: {
          owner: owner.publicKey,
          rewarder,
          rewardAuthority,
          stakeAccount,
          rewardMint: rewardMint.publicKey,
          rewardAccount: rewardTokenAccount,
          tokenProgram: splToken.TOKEN_PROGRAM_ID,
          clock: clockSysvar,
        },
        signers: [owner]
      });

      const rewardTokenAccountData = await rewardMint.getAccountInfo(rewardTokenAccount);
      expect(rewardTokenAccountData.amount.toNumber()).to.equal(seconds * rewardRate);
    });

    it('unstakes an NFT', async () => {
      //sleep one more second to check that we claim pending rewards on unstake
      await sleep(provider.connection, 1);

      await gmootStakingProgram.rpc.unstakeGmoot({
        accounts: {
          owner: owner.publicKey,
          rewarder,
          rewardAuthority,
          stakeAccount,
          rewardMint: rewardMint.publicKey,
          rewardTokenAccount,
          nftMint: nftMint.publicKey,
          nftTokenAccount,
          nftVault,
          tokenProgram: splToken.TOKEN_PROGRAM_ID,
          clock: clockSysvar,
        },
        signers: [owner],
      });
      const rewardTokenAccountData = await rewardMint.getAccountInfo(rewardTokenAccount);
      expect(rewardTokenAccountData.amount.toNumber()).to.equal(3 * rewardRate);
    });

  });

});

// Polls the network and returns once the block time has increased by seconds.
const sleep = async (connection: anchor.web3.Connection, seconds: number, startTime: number | null = null) => {
  let time = startTime;
  if (time == null) {
    let slot = await connection.getSlot();
    time = await connection.getBlockTime(slot);
  }
  let elapsed = 0;
  while (elapsed < seconds) {
    let slot = await connection.getSlot();
    let newTime = await connection.getBlockTime(slot);
    elapsed += newTime - time;
    time = newTime;
  }
};
