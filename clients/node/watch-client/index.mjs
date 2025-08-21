import grpc from '@grpc/grpc-js';
import protoLoader from '@grpc/proto-loader';

const PROTO_PATH = new URL('../../../proto/agentstate.proto', import.meta.url).pathname;
const packageDefinition = protoLoader.loadSync(PROTO_PATH, { keepCase: true, longs: String, enums: String, defaults: true });
const agent = grpc.loadPackageDefinition(packageDefinition).agentstate.v1;

const ns = process.argv[2] || 'acme';
let fromCommit = parseInt(process.argv[3] || '0', 10);
const endpoint = process.env.GRPC_ENDPOINT || 'localhost:9090';

function run() {
  const client = new agent.AgentState(endpoint, grpc.credentials.createInsecure());
  const call = client.Watch({ ns, from_commit: fromCommit });
  call.on('data', (ev) => {
    console.log(ev.type, ev.commit, ev.id);
    fromCommit = ev.commit;
  });
  call.on('error', (err) => {
    console.error('stream err', err.message);
    setTimeout(run, 1000);
  });
  call.on('end', () => {
    console.log('ended');
    setTimeout(run, 1000);
  });
}

run();

