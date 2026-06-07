import fetch from 'node-fetch';
import * as fs from 'fs';
import { Parser } from 'json2csv';

const SUBGRAPH_URL = 'https://api.goldsky.com/api/public/project_cm7frczqlbuzv010qd0ahdjob/subgraphs/culttokens-monad-testnet/v1/gn';
const BATCH_SIZE = 1000;

interface TokenClaimed {
  id: string;
  block_number: string;
  timestamp_: string;
  transactionHash_: string;
  contractId_: string;
  token: string;
  recipient: string;
  amount: string;
}

// Utility: Format current date like '29July2025'
function getFormattedDate(): string {
  const date = new Date();
  const day = date.getDate();
  const year = date.getFullYear();
  const monthNames = [
    'January', 'February', 'March', 'April', 'May', 'June',
    'July', 'August', 'September', 'October', 'November', 'December'
  ];
  const month = monthNames[date.getMonth()];
  return `${day}${month}${year}`;
}

async function fetchBatch(skip: number): Promise<TokenClaimed[]> {
  const query = `
    {
      tokensClaimeds(first: ${BATCH_SIZE}, skip: ${skip}) {
        id
        block_number
        timestamp_
        transactionHash_
        contractId_
        token
        recipient
        amount
      }
    }
  `;

  const response = await fetch(SUBGRAPH_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ query }),
  });

  const json = await response.json();

  if (json.errors) {
    console.error('GraphQL error:', json.errors);
    return [];
  }

  return json.data.tokensClaimeds as TokenClaimed[];
}

async function fetchAll() {
  let allData: TokenClaimed[] = [];
  let skip = 0;

  while (true) {
    console.log(`Fetching batch with skip=${skip}...`);
    const batch = await fetchBatch(skip);

    if (batch.length === 0) {
      console.log('No more data to fetch.');
      break;
    }

    allData = allData.concat(batch);
    skip += BATCH_SIZE;
  }

  const dateStr = getFormattedDate();
  const jsonFilename = `tokensClaimed_${dateStr}.json`;
  const csvFilename = `tokensClaimed_${dateStr}.csv`;

  // Save JSON
  fs.writeFileSync(jsonFilename, JSON.stringify(allData, null, 2));
  console.log(`✅ Saved ${allData.length} records to ${jsonFilename}`);

  // Save CSV
  const parser = new Parser();
  const csv = parser.parse(allData);
  fs.writeFileSync(csvFilename, csv);
  console.log(`✅ Saved ${allData.length} records to ${csvFilename}`);
}

fetchAll().catch((err) => console.error('❌ Fetch failed:', err));

