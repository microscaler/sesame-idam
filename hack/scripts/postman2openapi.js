const postmanToOpenApi = require('postman-to-openapi')

const currentPath = process.cwd();
console.log(`Current execution path: ${currentPath}`);

const postmanCollection = process.argv[2] || "{currentPath}/collection.json";
const outputFile = process.argv[3] || "${currentPath}/openapi.json";

async function convertCollection() {
    try {
        const result = await postmanToOpenApi(postmanCollection, outputFile, {
            defaultTag: 'MyAPI',
            outputFormat: 'json',
            includeAuthInfoInExample: true
        })
        console.log(`OpenAPI specs: ${result}`)
    } catch (err) {
        console.error('Conversion failed:', err)
    }
}

convertCollection()