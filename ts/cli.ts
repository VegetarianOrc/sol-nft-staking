import * as anchor from "@project-serum/anchor";
import { Program, web3 } from "@project-serum/anchor";
import { SolNftStaking } from "../target/types/sol_nft_staking";
import { Command, program as cliProgram } from "commander";
import * as fs from "fs";
import * as splToken from "@solana/spl-token";

const SOL_NFT_STAKING_PROGRAM_ID = new web3.PublicKey(
  "3zPPaZhN3tAkSJhjcEcyT7kAM6b2stQmJf65Fw9sMZa3"
);

const systemProgram = anchor.web3.SystemProgram.programId;
const rentSysvar = anchor.web3.SYSVAR_RENT_PUBKEY;
const clockSysvar = anchor.web3.SYSVAR_CLOCK_PUBKEY;

function cliCommand(name: string): Command {
  return cliProgram
    .command(name)
    .option(
      "-e, --env <string>",
      "Solana cluster env name",
      "devnet" //mainnet-beta, testnet, devnet
    )
    .option(
      "-k, --keypair <path>",
      `Solana wallet location`,
      "--keypair not provided"
    );
}

function loadWalletKey(keypair): web3.Keypair {
  if (!keypair || keypair == "") {
    throw new Error("Keypair is required!");
  }
  const loaded = web3.Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(fs.readFileSync(keypair).toString()))
  );
  console.log(`wallet public key: ${loaded.publicKey}`);
  return loaded;
}

interface Creator {
  address: web3.PublicKey;
  verified: boolean;
  share: number;
}

function loadCreators(creators): Creator[] {
  let parsedCreators = [];

  return parsedCreators;
}

async function getRewarderAddress(
  collectionName: string
): Promise<[web3.PublicKey, number]> {
  return await anchor.web3.PublicKey.findProgramAddress(
    [
      Buffer.from(collectionName),
      SOL_NFT_STAKING_PROGRAM_ID.toBuffer(),
      Buffer.from("rewarder"),
    ],
    SOL_NFT_STAKING_PROGRAM_ID
  );
}

async function getRewarderAuthority(
  collectionName: string,
  rewarderAddress: web3.PublicKey
): Promise<[web3.PublicKey, number]> {
  return await anchor.web3.PublicKey.findProgramAddress(
    [
      Buffer.from(collectionName),
      SOL_NFT_STAKING_PROGRAM_ID.toBuffer(),
      Buffer.from("rewarder"),
      rewarderAddress.toBuffer(),
    ],
    SOL_NFT_STAKING_PROGRAM_ID
  );
}

function printRewarder(address: web3.PublicKey, rewarder: any) {
  const toLog = {
    authority: rewarder.authority.toBase58(),
    rewardMint: rewarder.rewardMint.toBase58(),
    rewardAuthorityBump: rewarder.rewardAuthorityBump,
    rewardRate: rewarder.rewardRate.toNumber(),
    allowedUpdateAuthority: rewarder.allowedUpdateAuthority.toBase58(),
    creators: rewarder.creators,
    collection: rewarder.collection,
    enforceMetadata: rewarder.enforceMetadata,
    totalStaked: rewarder.totalStaked,
  };
  console.log(
    `Rewarder ${address.toBase58()}\n${JSON.stringify(toLog, null, 2)}`
  );
}

cliProgram.version("0.0.1");

const rewarderCommand = cliCommand("rewarder");

rewarderCommand
  .command("get")
  .option("-a, --address <string>", "The address of the rewarder", null)
  .option(
    "-n, --name <string>",
    "The name of the NFT collection the rewarder is for",
    null
  )
  .action(async (directory: string, cmd: Command) => {
    const { env } = cmd.parent.opts();
    const { name, address } = cmd.opts();
    const connection = new web3.Connection(web3.clusterApiUrl(env));

    if ((!address || address.length === 0) && (!name || name.length === 0)) {
      console.log("Either address or name is required");
      return;
    }

    let key;
    if (address && address.length !== 0) {
      key = new web3.PublicKey(address);
    } else {
      const [rewarderAddress, _] = await getRewarderAddress(name);
      key = rewarderAddress;
    }

    anchor.setProvider(
      new anchor.Provider(connection, null, { commitment: "confirmed" })
    );

    const solNftStakingProgram = (await Program.at(
      SOL_NFT_STAKING_PROGRAM_ID
    )) as Program<SolNftStaking>;

    const rewarder = await solNftStakingProgram.account.nftStakeRewarder.fetch(
      key
    );
    printRewarder(key, rewarder);
  });

rewarderCommand
  .command("create")
  .requiredOption(
    "-d, --decimals <number>",
    "The number of decimals for the reward token"
  )
  .requiredOption(
    "-r, --rewardRate <number>",
    "The number reward per second per nft staked for the rewarder"
  )
  .requiredOption(
    "-n, --name <string>",
    "The name of the NFT collection the rewarder is for"
  )
  .option(
    "-c, --creators <path>",
    "the path to a json array of nft creator objects"
  )
  .action(async (directory: string, cmd: Command) => {
    const { env, keypair } = cmd.parent.opts();
    const { decimals, name, rewardRate, creators } = cmd.opts();
    const collectionName = name;
    const connection = new web3.Connection(web3.clusterApiUrl(env));
    const walletKeyPair = loadWalletKey(keypair);
    const wallet = new anchor.Wallet(walletKeyPair);

    anchor.setProvider(
      new anchor.Provider(connection, wallet, {
        commitment: "confirmed",
      })
    );

    const parsedCreators = loadCreators(creators);

    const solNftStakingProgram = (await Program.at(
      SOL_NFT_STAKING_PROGRAM_ID
    )) as Program<SolNftStaking>;
    console.log(`Creating rewarder for '${name}'`);

    console.log(`Finding PDAs for rewarder and mint authority`);
    const [rewarder, rewarderBump] = await getRewarderAddress(collectionName);
    const [rewardAuthority, rewardAuthorityBump] = await getRewarderAuthority(
      collectionName,
      rewarder
    );
    console.log(`creating reward mint`);
    const rewardMint = await splToken.Token.createMint(
      connection,
      walletKeyPair, //payer
      rewardAuthority, //mint authority
      null, //freeze authority
      decimals, //deicmals
      splToken.TOKEN_PROGRAM_ID
    );
    console.log(`Reward mint created: ${rewardMint.publicKey.toBase58()} `);

    const initRewarderTxId = await solNftStakingProgram.rpc.initializeRewarder(
      rewarderBump,
      rewardAuthorityBump,
      new anchor.BN(rewardRate),
      Buffer.from(collectionName),
      parsedCreators,
      wallet.publicKey,
      false,
      {
        accounts: {
          rewarder: rewarder,
          authority: wallet.publicKey,
          rewardAuthority: rewardAuthority,
          rewardMint: rewardMint.publicKey,
          systemProgram,
          rent: rentSysvar,
        },
        signers: [walletKeyPair],
      }
    );

    await connection.confirmTransaction(initRewarderTxId, "confirmed");
    console.log(
      `Rewarder created. Address: ${rewarder.toBase58()}, creation transaction ID: ${initRewarderTxId}`
    );
  });

cliProgram.parse(process.argv);
