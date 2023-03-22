import { PrismaClient } from '@prisma/client'
import { env } from 'process';

const prisma = new PrismaClient()

export default prisma;

import mainnet from './mainnet.data.json';
import testnet from './testnet.data.json';
import testnet2 from './testnet2.data.json';

let data: { badge?: string, project?: string; minter?: string; vester?: string; offseter?: string; yielder?: string; }[];
switch (env.NETWORK) {
    case 'testnet':
        data = testnet;
        break;
    case 'testnet2':
        data = testnet2;
        break;
    default:
        data = mainnet;
}

export { data };
