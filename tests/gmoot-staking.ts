import * as anchor from '@project-serum/anchor';
import { Program } from '@project-serum/anchor';
import { GmootStaking } from '../target/types/gmoot_staking';

describe('gmoot-staking', () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.GmootStaking as Program<GmootStaking>;

  it('Is initialized!', async () => {
    // Add your test here.
    const tx = await program.rpc.initialize({});
    console.log("Your transaction signature", tx);
  });
});
