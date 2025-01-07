use std::rc::Rc;

use ethabi::Token;
use zksync_contracts::l1_messenger_contract;
use zksync_test_contracts::{TestContract, TxType};
use zksync_types::{
    address_to_h256, u256_to_h256, web3::keccak256, Address, Execute, ProtocolVersionId,
    L1_MESSENGER_ADDRESS, U256,
};
use zksync_vm_interface::SystemEnv;

use super::{default_system_env, ContractToDeploy, TestedVm, VmTesterBuilder};
use crate::{
    interface::{
        pubdata::{PubdataBuilder, PubdataInput},
        InspectExecutionMode, TxExecutionMode, VmInterfaceExt,
    },
    pubdata_builders::FullPubdataBuilder,
    vm_latest::constants::ZK_SYNC_BYTES_PER_BLOB,
};

const L2_DA_VALIDATOR_OUTPUT_HASH_KEY: usize = 5;
const USED_L2_DA_VALIDATOR_ADDRESS_KEY: usize = 6;

// Bytecode is temporary hardcoded, should be removed after contracts are merged.
fn l2_rollup_da_validator_bytecode() -> Vec<u8> {
    hex::decode("0012000000000002000a000000000002000000000301001900000060043002700000012703400197000100000031035500020000003103550003000000310355000400000031035500050000003103550006000000310355000700000031035500080000003103550009000000310355000a000000310355000b000000310355000c000000310355000d000000310355000e000000310355000f00000031035500100000003103550011000000010355000001270040019d0000008004000039000000400040043f00000001002001900000005d0000c13d000000040030008c000000fe0000413d000000000201043b00000129022001970000012a0020009c000000fe0000c13d000000a40030008c000000fe0000413d0000000002000416000000000002004b000000fe0000c13d0000008402100370000000000202043b000300000002001d0000012b0020009c000000fe0000213d00000003020000290000002302200039000000000032004b000000fe0000813d00000003020000290000000402200039000000000421034f000000000604043b0000012b0060009c000000fe0000213d0000000304000029000700240040003d0000000704600029000000000034004b000000fe0000213d0000004403100370000000000303043b000400000003001d0000006403100370000000000303043b000200000003001d000000040060008c000000fe0000413d0000002002200039000000000221034f000000000202043b000000e00220027000000058022000c90000000804200039000000000064004b000000fe0000213d00000003022000290000002802200039000000000121034f000000000101043b000500e00010027a000600000006001d000000650000c13d00000000090000190000000403000029000000000039004b000000f10000c13d0000014e0040009c000000fb0000a13d0000014001000041000000000010043f0000001101000039000000040010043f00000138010000410000049a000104300000000001000416000000000001004b000000fe0000c13d0000002001000039000001000010044300000120000004430000012801000041000004990001042e000000000800001900000000090000190000014f0040009c000000570000813d0000000403400039000000000063004b000000fe0000213d00000007024000290000001101000367000000000221034f000000000502043b000000e004500270000000000034001a000000570000413d0000000007340019000000000067004b000000fe0000213d00000000020004140000012c0050009c0000007b0000813d0000000003000031000000840000013d000000070330002900000127053001970001000000510355000000000034001a000000570000413d0000000003340019000000000330007b000000570000413d000000000151034f000a00000009001d000800000008001d000900000007001d000001270330019700010000003103e50000012d0020009c000003c20000813d00000000013103df000000c0022002100000012e022001970000012c022001c700010000002103b500000000012103af0000801002000039049804930000040f0000000003010019000000600330027000000127033001970000000100200190000002450000613d0000001f0230003900000130052001970000003f025000390000013104200197000000400200043d0000000004420019000000000024004b000000000600003900000001060040390000012b0040009c0000023f0000213d00000001006001900000023f0000c13d000000400040043f0000000004320436000000000005004b000000b10000613d0000000005540019000000000600003100000011066003670000000007040019000000006806043c0000000007870436000000000057004b000000ad0000c13d0000012f063001980000000005640019000000ba0000613d000000000701034f0000000008040019000000007907043c0000000008980436000000000058004b000000b60000c13d0000001f03300190000000c70000613d000000000161034f0000000303300210000000000605043300000000063601cf000000000636022f000000000101043b0000010003300089000000000131022f00000000013101cf000000000161019f00000000001504350000000001020433000000200010008c0000000a05000029000004210000c13d0000000002040433000000400100043d000000400310003900000000002304350000002002100039000000000052043500000040030000390000000000310435000001320010009c0000023f0000213d0000006003100039000000400030043f000001270020009c000001270200804100000040022002100000000001010433000001270010009c00000127010080410000006001100210000000000121019f0000000002000414000001270020009c0000012702008041000000c002200210000000000112019f00000133011001c700008010020000390498048e0000040f0000000100200190000000060600002900000009040000290000000808000029000000fe0000613d000000000901043b0000000108800039000000050080006c000000670000413d000000520000013d000000400100043d000000440210003900000000009204350000002402100039000000000032043500000134020000410000000000210435000000040210003900000000000204350000042d0000013d0000000403400039000000000063004b000001000000a13d00000000010000190000049a0001043000000007014000290000001101100367000000000101043b000400e00010027a0000025d0000c13d000000000900001900000000050300190000000003090019000000020090006c000002f20000c13d000000060050006c000002fd0000813d00000007015000290000001102000367000000000112034f000000000101043b000000f801100270000000010010008c000003030000c13d00000000060500190000014e0060009c0000000604000029000000570000213d0000000403600039000000000043004b000000fe0000213d00000003016000290000002501100039000000000112034f000000000101043b000000000043004b000002fd0000813d000000e8011002700000000703300029000000000432034f0000000503500039000000000404043b000000000031001a0000000607000029000000570000413d000a00000031001d0000000a0070006b000000fe0000213d000000050600008a0000000a0060006b000000570000213d0000000a050000290000000405500039000000000075004b000000fe0000213d0000000a08000029000300070080002d0000000306200360000000000606043b000400000006001d000000e006600272000500000006001d00090110006000cd0000013f0000613d000000090800002900000005068000fa000001100060008c000000570000c13d000000090050002a000000570000413d000200090050002d000000020070006c000000fe0000413d000000f804400270000000400a00043d0000004406a00039000000800700003900000000007604350000002406a000390000000000460435000001410400004100000000004a043500000007055000290000008404a00039000000090900002900000000009404350000000404a0003900000005060000290000000000640435000000000752034f0000001f0890018f00080000000a001d000000a405a0003900000142099001980000000006950019000001610000613d000000000a07034f000000000b05001900000000ac0a043c000000000bcb043600000000006b004b0000015d0000c13d0000000703300029000000000008004b0000016f0000613d000000000797034f0000000308800210000000000906043300000000098901cf000000000989022f000000000707043b0000010008800089000000000787022f00000000078701cf000000000797019f00000000007604350000000907000029000000000675001900000000000604350000001f06700039000001430660019700000000066500190000000004460049000000080500002900000064055000390000000000450435000000000432034f0000001f0510018f000000000216043600000144061001980000000003620019000001850000613d000000000704034f0000000008020019000000007907043c0000000008980436000000000038004b000001810000c13d000000000005004b000001920000613d000000000464034f0000000305500210000000000603043300000000065601cf000000000656022f000000000404043b0000010005500089000000000454022f00000000045401cf000000000464019f0000000000430435000000000312001900000000000304350000001f011000390000014501100197000000080300002900000000013100490000000001210019000001270010009c00000127010080410000006001100210000001270030009c000001270200004100000000020340190000004002200210000000000121019f0000000002000414000001270020009c0000012702008041000000c002200210000000000112019f0000800e02000039049804890000040f000000000301001900000060033002700000012703300197000000200030008c000000200400003900000000040340190000001f0640018f00000020074001900000000805700029000001b80000613d000000000801034f0000000809000029000000008a08043c0000000009a90436000000000059004b000001b40000c13d000000000006004b000001c50000613d000000000771034f0000000306600210000000000805043300000000086801cf000000000868022f000000000707043b0000010006600089000000000767022f00000000066701cf000000000686019f00000000006504350000000100200190000003480000613d0000001f01400039000000600110018f0000000802100029000000000012004b00000000010000390000000101004039000100000002001d0000012b0020009c0000023f0000213d00000001001001900000023f0000c13d0000000101000029000000400010043f000000200030008c0000000604000029000000fe0000413d00000008010000290000000001010433000800000001001d00000004010000290000012c0010009c000001e10000413d000000090200002900000005012000fa000001100010008c000000570000c13d0000000103000029000000440130003900000024023000390000000403300039000000020440006c000003660000c13d000001460400004100000001050000290000000000450435000000200400003900000000004304350000000a04000029000000000042043500000150034001980000001f0440018f000000000231001900000007050000290000001105500367000001fa0000613d000000000605034f0000000007010019000000006806043c0000000007870436000000000027004b000001f60000c13d000000000004004b000002070000613d000000000335034f0000000304400210000000000502043300000000054501cf000000000545022f000000000303043b0000010004400089000000000343022f00000000034301cf000000000353019f00000000003204350000000a030000290000001f023000390000015002200197000000000131001900000000000104350000004401200039000001270010009c000001270100804100000060011002100000000102000029000001270020009c00000127020080410000004002200210000000000112019f0000000002000414000001270020009c0000012702008041000000c002200210000000000112019f00008011020000390498048e0000040f000000000301001900000060033002700000001f0430018f0000012f0530019700000127033001970000000100200190000003720000613d0000000102500029000000000005004b0000022c0000613d000000000601034f0000000107000029000000006806043c0000000007870436000000000027004b000002280000c13d000000000004004b000002390000613d000000000151034f0000000304400210000000000502043300000000054501cf000000000545022f000000000101043b0000010004400089000000000141022f00000000014101cf000000000151019f00000000001204350000001f0130003900000130011001970000000101100029000900000001001d0000012b0010009c0000038a0000a13d0000014001000041000000000010043f0000004101000039000000040010043f00000138010000410000049a000104300000001f0430018f0000012f023001980000024e0000613d000000000501034f0000000006000019000000005705043c0000000006760436000000000026004b0000024a0000c13d000000000004004b0000025b0000613d000000000121034f0000000304400210000000000502043300000000054501cf000000000545022f000000000101043b0000010004400089000000000141022f00000000014101cf000000000151019f000000000012043500000060013002100000049a00010430000000000800001900000000090000190000014e0030009c000000570000213d0000000402300039000000000062004b000000fe0000213d00000007033000290000001101000367000000000331034f000000000303043b000000e00a30027000000000002a001a000000570000413d00000000072a0019000000000067004b000000fe0000213d0000013600300198000003130000c13d000001390030009c000003190000813d0000013a003001980000031f0000613d000000070420002900000127034001970000000002000414000100000031035500000000004a001a000000570000413d00000000044a0019000000000440007b000000570000413d00090000000a001d000a00000009001d000500000008001d000800000007001d000000000131034f000001270340019700010000003103e5000001270020009c000003c20000213d00000000013103df000000c0022002100000012e022001970000012c022001c700010000002103b500000000012103af0000000202000039049804930000040f00000000030100190000006003300270000001270330019700000001002001900000032a0000613d0000001f0230003900000130052001970000003f025000390000013104200197000000400200043d0000000004420019000000000024004b000000000600003900000001060040390000012b0040009c0000023f0000213d00000001006001900000023f0000c13d000000400040043f0000000004320436000000000005004b000000090a000029000002ad0000613d0000000005540019000000000600003100000011066003670000000007040019000000006806043c0000000007870436000000000057004b000002a90000c13d0000012f063001980000000005640019000002b60000613d000000000701034f0000000008040019000000007907043c0000000008980436000000000058004b000002b20000c13d0000001f03300190000002c30000613d000000000161034f0000000303300210000000000605043300000000063601cf000000000636022f000000000101043b0000010003300089000000000131022f00000000013101cf000000000161019f0000000000150435000000400100043d0000000002020433000000200020008c0000000a05000029000003420000c13d00000000020404330000013d02200197000000db03a002100000013e03300197000000000223019f0000013f022001c7000000400310003900000000002304350000002002100039000000000052043500000040030000390000000000310435000001320010009c0000023f0000213d0000006003100039000000400030043f000001270020009c000001270200804100000040022002100000000001010433000001270010009c00000127010080410000006001100210000000000121019f0000000002000414000001270020009c0000012702008041000000c002200210000000000112019f00000133011001c700008010020000390498048e0000040f0000000100200190000000060600002900000008030000290000000508000029000000fe0000613d000000000901043b0000000108800039000000040080006c0000025f0000413d000001060000013d000000400100043d0000004402100039000000000032043500000024021000390000000203000029000000000032043500000134020000410000000000210435000000040210003900000001030000390000042c0000013d0000014001000041000000000010043f0000003201000039000000040010043f00000138010000410000049a00010430000000400200043d0000004403200039000000000013043500000024012000390000000103000039000000000031043500000134010000410000000000120435000000040120003900000002030000390000000000310435000001270020009c0000012702008041000000400120021000000135011001c70000049a00010430000000400100043d0000013702000041000000000021043500000004021000390000000203000039000003240000013d000000400100043d0000013702000041000000000021043500000004021000390000000103000039000003240000013d000000400100043d00000137020000410000000000210435000000040210003900000003030000390000000000320435000001270010009c0000012701008041000000400110021000000138011001c70000049a000104300000001f0430018f0000012f02300198000003330000613d000000000501034f0000000006000019000000005705043c0000000006760436000000000026004b0000032f0000c13d000000000004004b000003400000613d000000000121034f0000000304400210000000000502043300000000054501cf000000000545022f000000000101043b0000010004400089000000000141022f00000000014101cf000000000151019f000000000012043500000060013002100000049a0001043000000044021000390000013b03000041000000000032043500000024021000390000001903000039000004270000013d0000001f0530018f0000012f06300198000000400200043d0000000004620019000003530000613d000000000701034f0000000008020019000000007907043c0000000008980436000000000048004b0000034f0000c13d000000000005004b000003600000613d000000000161034f0000000305500210000000000604043300000000065601cf000000000656022f000000000101043b0000010005500089000000000151022f00000000015101cf000000000161019f00000000001404350000006001300210000001270020009c00000127020080410000004002200210000000000112019f0000049a000104300000013405000041000000010600002900000000005604350000000305000039000000000053043500000000000204350000000000410435000001270060009c0000012706008041000000400160021000000135011001c70000049a00010430000000400200043d0000000006520019000000000005004b0000037c0000613d000000000701034f0000000008020019000000007907043c0000000008980436000000000068004b000003780000c13d000000000004004b000003600000613d000000000151034f0000000304400210000000000506043300000000054501cf000000000545022f000000000101043b0000010004400089000000000141022f00000000014101cf000000000151019f0000000000160435000003600000013d0000000901000029000000400010043f000000200030008c000000fe0000413d000000010100002900000000010104330000012b0010009c000000fe0000213d000000010230002900000001011000290000001f03100039000000000023004b000000fe0000813d00000000140104340000012b0040009c0000023f0000213d00000005034002100000003f05300039000001470550019700000009055000290000012b0050009c0000023f0000213d000000400050043f000000090500002900000000004504350000000003130019000000000023004b000000fe0000213d000000000004004b000003ae0000613d0000000902000029000000200220003900000000140104340000000000420435000000000031004b000003a90000413d000000000100041400000011020003670000000a0000006b000003b40000c13d0000000003000031000003be0000013d00000007030000290000012704300197000100000042035500000003050000290000000a0050006c000000570000413d0000000305000029000000000350007b000000570000413d000000000242034f000001270330019700010000003203e5000001270010009c000003c90000a13d000000400100043d00000044021000390000014d03000041000000000032043500000024021000390000000803000039000004270000013d00000000023203df000000c0011002100000012e011001970000012c011001c700010000001203b500000000011203af0000801002000039049804930000040f0000000003010019000000600330027000000127033001970000000100200190000004320000613d0000001f0230003900000130052001970000003f025000390000013104200197000000400200043d0000000004420019000000000024004b000000000600003900000001060040390000012b0040009c0000023f0000213d00000001006001900000023f0000c13d000000400040043f0000000004320436000000000005004b000003ef0000613d0000000005540019000000000600003100000011066003670000000007040019000000006806043c0000000007870436000000000057004b000003eb0000c13d0000001f0530018f0000012f063001980000000003640019000003f90000613d000000000701034f0000000008040019000000007907043c0000000008980436000000000038004b000003f50000c13d000000000005004b000004060000613d000000000161034f0000000305500210000000000603043300000000065601cf000000000656022f000000000101043b0000010005500089000000000151022f00000000015101cf000000000161019f00000000001304350000000001020433000000200010008c000004210000c13d000000400100043d00000009020000290000000002020433000001000020008c0000044a0000413d00000064021000390000014a03000041000000000032043500000044021000390000014b0300004100000000003204350000002402100039000000250300003900000000003204350000013c020000410000000000210435000000040210003900000020030000390000000000320435000001270010009c000001270100804100000040011002100000014c011001c70000049a00010430000000400100043d00000044021000390000014803000041000000000032043500000024021000390000001f0300003900000000003204350000013c020000410000000000210435000000040210003900000020030000390000000000320435000001270010009c0000012701008041000000400110021000000135011001c70000049a000104300000001f0430018f0000012f023001980000043b0000613d000000000501034f0000000006000019000000005705043c0000000006760436000000000026004b000004370000c13d000000000004004b000004480000613d000000000121034f0000000304400210000000000502043300000000054501cf000000000545022f000000000101043b0000010004400089000000000141022f00000000014101cf000000000151019f000000000012043500000060013002100000049a000104300000000003040433000000f8022002100000006004100039000000000024043500000040021000390000000000320435000000200210003900000008030000290000000000320435000000610310003900000009040000290000000004040433000000000004004b000004610000613d000000000500001900000009060000290000002006600039000900000006001d000000000606043300000000036304360000000105500039000000000045004b000004590000413d0000000003130049000000200430008a00000000004104350000001f0330003900000150043001970000000003140019000000000043004b000000000400003900000001040040390000012b0030009c0000023f0000213d00000001004001900000023f0000c13d000000400030043f000001270020009c000001270200804100000040022002100000000001010433000001270010009c00000127010080410000006001100210000000000121019f0000000002000414000001270020009c0000012702008041000000c002200210000000000112019f00000133011001c700008010020000390498048e0000040f0000000100200190000000fe0000613d000000000101043b000000400200043d0000000000120435000001270020009c0000012702008041000000400120021000000149011001c7000004990001042e0000048c002104210000000102000039000000000001042d0000000002000019000000000001042d00000491002104230000000102000039000000000001042d0000000002000019000000000001042d00000496002104230000000102000039000000000001042d0000000002000019000000000001042d0000049800000432000004990001042e0000049a00010430000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000000200000000000000000000000000000040000001000000000000000000ffffffff0000000000000000000000000000000000000000000000000000000089f9a07200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffffffffffff0000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000ffffffff00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffe000000000000000000000000000000000000000000000000000000001ffffffe000000000000000000000000000000000000000000000000000000003ffffffe0000000000000000000000000000000000000000000000000ffffffffffffff9f02000000000000000000000000000000000000000000000000000000000000007f7b0cf70000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000640000000000000000000000000000001f0000000000000000000000000000000000000000000000000000000043e266b0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000024000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000007368612072657475726e656420696e76616c696420646174610000000000000008c379a00000000000000000000000000000000000000000000000000000000000000000ffffffffffffffffffffffffffffffffffffffffffffffffffffffff06ffffff0000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000004e487b71000000000000000000000000000000000000000000000000000000006006d8b500000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001ffffffffe0000000000000000000000000000000000000000000000000000003ffffffffe00000000000000000000000000000000000000000000000000000000000ffffe00000000000000000000000000000000000000000000000000000000001ffffe018876a04000000000000000000000000000000000000000000000000000000007fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe06b656363616b3235362072657475726e656420696e76616c69642064617461000000000000000000000000000000000000000020000000000000000000000000206269747300000000000000000000000000000000000000000000000000000053616665436173743a2076616c756520646f65736e27742066697420696e203800000000000000000000000000000000000000840000000000000000000000004f766572666c6f77000000000000000000000000000000000000000000000000fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffbfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffcffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe00000000000000000000000000000000000000000000000000000000000000000e901f5bd8811df26e614332e2110b9bc002e2cbadd82065c67e102f858079d5a").unwrap()
}

fn encoded_uncompressed_state_diffs(input: &PubdataInput) -> Vec<u8> {
    let mut result = vec![];
    for state_diff in input.state_diffs.iter() {
        result.extend(state_diff.encode_padded());
    }
    result
}

fn compose_header_for_l1_commit_rollup(input: PubdataInput) -> Vec<u8> {
    // The preimage under the hash `l2DAValidatorOutputHash` is expected to be in the following format:
    // - First 32 bytes are the hash of the uncompressed state diff.
    // - Then, there is a 32-byte hash of the full pubdata.
    // - Then, there is the 1-byte number of blobs published.
    // - Then, there are linear hashes of the published blobs, 32 bytes each.

    let mut full_header = vec![];

    let uncompressed_state_diffs = encoded_uncompressed_state_diffs(&input);
    let uncompressed_state_diffs_hash = keccak256(&uncompressed_state_diffs);
    full_header.extend(uncompressed_state_diffs_hash);

    let pubdata_builder = FullPubdataBuilder::new(Address::zero());
    let mut full_pubdata =
        pubdata_builder.settlement_layer_pubdata(&input, ProtocolVersionId::latest());
    let full_pubdata_hash = keccak256(&full_pubdata);
    full_header.extend(full_pubdata_hash);

    // Now, we need to calculate the linear hashes of the blobs.
    // Firstly, let's pad the pubdata to the size of the blob.
    if full_pubdata.len() % ZK_SYNC_BYTES_PER_BLOB != 0 {
        full_pubdata.resize(
            full_pubdata.len() + ZK_SYNC_BYTES_PER_BLOB
                - full_pubdata.len() % ZK_SYNC_BYTES_PER_BLOB,
            0,
        );
    }
    full_header.push((full_pubdata.len() / ZK_SYNC_BYTES_PER_BLOB) as u8);

    full_pubdata
        .chunks(ZK_SYNC_BYTES_PER_BLOB)
        .for_each(|chunk| {
            full_header.extend(keccak256(chunk));
        });

    full_header
}

pub(crate) fn test_rollup_da_output_hash_match<VM: TestedVm>() {
    // In this test, we check whether the L2 DA output hash is as expected.

    let l2_da_validator_address = Address::repeat_byte(0x12);
    let system_env = SystemEnv {
        version: ProtocolVersionId::Version27,
        ..default_system_env()
    };
    let mut vm = VmTesterBuilder::new()
        .with_empty_in_memory_storage()
        .with_execution_mode(TxExecutionMode::VerifyExecute)
        .with_rich_accounts(1)
        .with_system_env(system_env)
        .with_custom_contracts(vec![ContractToDeploy {
            bytecode: l2_rollup_da_validator_bytecode(),
            address: l2_da_validator_address,
            is_account: false,
            is_funded: false,
        }])
        .build::<VM>();

    let account = &mut vm.rich_accounts[0];

    // Firstly, deploy tx. It should publish the bytecode of the "test contract"
    let counter_bytecode = TestContract::counter().bytecode;
    let tx = account.get_deploy_tx(counter_bytecode, None, TxType::L2).tx;
    // We do not use compression here, to have the bytecode published in full.
    let (_, result) = vm
        .vm
        .execute_transaction_with_bytecode_compression(tx, false);
    assert!(!result.result.is_failed(), "Transaction wasn't successful");

    // Then, we call the l1 messenger to also send an L2->L1 message.
    let l1_messenger_contract = l1_messenger_contract();
    let encoded_data = l1_messenger_contract
        .function("sendToL1")
        .unwrap()
        .encode_input(&[Token::Bytes(vec![])])
        .unwrap();

    let tx = account.get_l2_tx_for_execute(
        Execute {
            contract_address: Some(L1_MESSENGER_ADDRESS),
            calldata: encoded_data,
            value: U256::zero(),
            factory_deps: vec![],
        },
        None,
    );
    vm.vm.push_transaction(tx);
    let result = vm.vm.execute(InspectExecutionMode::OneTx);
    assert!(!result.result.is_failed(), "Transaction wasn't successful");

    let pubdata_builder = FullPubdataBuilder::new(l2_da_validator_address);
    let batch_result = vm.vm.finish_batch(Rc::new(pubdata_builder));
    assert!(
        !batch_result.block_tip_execution_result.result.is_failed(),
        "Transaction wasn't successful {:?}",
        batch_result.block_tip_execution_result.result
    );
    let pubdata_input = vm.vm.pubdata_input();

    // Just to double check that the test makes sense.
    assert!(!pubdata_input.user_logs.is_empty());
    assert!(!pubdata_input.l2_to_l1_messages.is_empty());
    assert!(!pubdata_input.published_bytecodes.is_empty());
    assert!(!pubdata_input.state_diffs.is_empty());

    let expected_header: Vec<u8> = compose_header_for_l1_commit_rollup(pubdata_input);

    let l2_da_validator_output_hash = batch_result
        .block_tip_execution_result
        .logs
        .system_l2_to_l1_logs
        .iter()
        .find(|log| log.0.key == u256_to_h256(L2_DA_VALIDATOR_OUTPUT_HASH_KEY.into()))
        .unwrap()
        .0
        .value;

    assert_eq!(
        l2_da_validator_output_hash,
        keccak256(&expected_header).into()
    );

    let l2_used_da_validator_address = batch_result
        .block_tip_execution_result
        .logs
        .system_l2_to_l1_logs
        .iter()
        .find(|log| log.0.key == u256_to_h256(USED_L2_DA_VALIDATOR_ADDRESS_KEY.into()))
        .unwrap()
        .0
        .value;

    assert_eq!(
        l2_used_da_validator_address,
        address_to_h256(&l2_da_validator_address)
    );
}
