import Foundation
#if canImport(StoreKit)
import StoreKit
#endif

private struct PurchasePayload: Encodable {
    let productId: String
    let transactionId: String
    let originalTransactionId: String?
    let receiptData: String
    let source: String
}

private enum BridgeError: Error, CustomStringConvertible {
    case invalidArgs(String)
    case unsupportedPlatform(String)
    case productNotFound(String)
    case purchasePending
    case purchaseCancelled
    case purchaseUnverified(String)
    case receiptMissing(String)
    case internalError(String)

    var description: String {
        switch self {
        case .invalidArgs(let msg):
            return "invalid args: \(msg)"
        case .unsupportedPlatform(let msg):
            return "unsupported platform: \(msg)"
        case .productNotFound(let productId):
            return "storekit product not found: \(productId)"
        case .purchasePending:
            return "purchase is pending"
        case .purchaseCancelled:
            return "purchase is cancelled by user"
        case .purchaseUnverified(let msg):
            return "purchase unverified: \(msg)"
        case .receiptMissing(let msg):
            return "receipt is unavailable: \(msg)"
        case .internalError(let msg):
            return "internal error: \(msg)"
        }
    }
}

private struct ParsedArgs {
    let productId: String
    let simulate: Bool
}

private struct Simulator {
    static func payload(productId: String) -> PurchasePayload {
        let safeProduct = productId
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .replacingOccurrences(of: " ", with: "")
        let millis = UInt64(Date().timeIntervalSince1970 * 1000)
        return PurchasePayload(
            productId: safeProduct,
            transactionId: "sim-storekit-\(safeProduct)-\(millis)",
            originalTransactionId: nil,
            receiptData: "sim_receipt:\(safeProduct):\(millis)",
            source: "iap_storekit_bridge_simulated"
        )
    }
}

private func parseArgs(_ argv: [String]) throws -> ParsedArgs {
    var productId: String?
    var simulate = false
    var idx = 0

    while idx < argv.count {
        let arg = argv[idx]
        switch arg {
        case "--product-id":
            guard idx + 1 < argv.count else {
                throw BridgeError.invalidArgs("--product-id requires a value")
            }
            let value = argv[idx + 1].trimmingCharacters(in: .whitespacesAndNewlines)
            if value.isEmpty {
                throw BridgeError.invalidArgs("--product-id cannot be empty")
            }
            productId = value
            idx += 2
        case "--simulate":
            simulate = true
            idx += 1
        case "--help", "-h":
            let usage = """
            Usage:
              iap-storekit-bridge --product-id <product_id> [--simulate]

            Options:
              --product-id <id>   App Store IAP product id (required)
              --simulate          Output simulated payload without StoreKit purchase
            """
            throw BridgeError.invalidArgs(usage)
        default:
            throw BridgeError.invalidArgs("unknown arg: \(arg)")
        }
    }

    guard let pid = productId else {
        throw BridgeError.invalidArgs("--product-id is required")
    }
    return ParsedArgs(productId: pid, simulate: simulate)
}

private func jsonString<T: Encodable>(_ value: T) throws -> String {
    let encoder = JSONEncoder()
    let data = try encoder.encode(value)
    guard let text = String(data: data, encoding: .utf8) else {
        throw BridgeError.internalError("failed to encode utf8 json")
    }
    return text
}

#if canImport(StoreKit)
@available(iOS 15.0, macOS 13.0, *)
private func verifiedTransaction(
    from result: VerificationResult<Transaction>
) throws -> Transaction {
    switch result {
    case .unverified(_, let error):
        throw BridgeError.purchaseUnverified(error.localizedDescription)
    case .verified(let transaction):
        return transaction
    }
}

@available(iOS 15.0, macOS 13.0, *)
private func loadReceiptData() throws -> String {
    if let override = ProcessInfo.processInfo.environment["AICOMM_IAP_RECEIPT_B64"] {
        let normalized = override.trimmingCharacters(in: .whitespacesAndNewlines)
        if !normalized.isEmpty {
            return normalized
        }
    }

    guard let receiptUrl = Bundle.main.appStoreReceiptURL else {
        throw BridgeError.receiptMissing("Bundle.main.appStoreReceiptURL is nil")
    }
    let data = try Data(contentsOf: receiptUrl)
    let encoded = data.base64EncodedString()
    if encoded.isEmpty {
        throw BridgeError.receiptMissing("receipt file exists but empty")
    }
    return encoded
}

@available(iOS 15.0, macOS 13.0, *)
private func performStoreKitPurchase(productId: String) async throws -> PurchasePayload {
    let products = try await Product.products(for: [productId])
    guard let product = products.first else {
        throw BridgeError.productNotFound(productId)
    }

    let purchaseResult = try await product.purchase()
    switch purchaseResult {
    case .pending:
        throw BridgeError.purchasePending
    case .userCancelled:
        throw BridgeError.purchaseCancelled
    case .success(let verification):
        let transaction = try verifiedTransaction(from: verification)
        let receiptData = try loadReceiptData()
        let originalId = transaction.originalID == transaction.id ? nil : String(transaction.originalID)
        let payload = PurchasePayload(
            productId: productId,
            transactionId: String(transaction.id),
            originalTransactionId: originalId,
            receiptData: receiptData,
            source: "iap_storekit_bridge_storekit2"
        )
        await transaction.finish()
        return payload
    @unknown default:
        throw BridgeError.internalError("unknown purchase result")
    }
}
#endif

@main
struct Main {
    static func main() async {
        do {
            let args = try parseArgs(Array(CommandLine.arguments.dropFirst()))
            let shouldSimulate = args.simulate || ProcessInfo.processInfo.environment["AICOMM_IAP_SIMULATE"] == "1"

            let payload: PurchasePayload
            if shouldSimulate {
                payload = Simulator.payload(productId: args.productId)
            } else {
#if canImport(StoreKit)
                if #available(iOS 15.0, macOS 13.0, *) {
                    payload = try await performStoreKitPurchase(productId: args.productId)
                } else {
                    throw BridgeError.unsupportedPlatform("requires iOS 15+ or macOS 13+")
                }
#else
                throw BridgeError.unsupportedPlatform("StoreKit framework unavailable on current platform")
#endif
            }

            let text = try jsonString(payload)
            FileHandle.standardOutput.write((text + "\n").data(using: .utf8)!)
            Foundation.exit(0)
        } catch {
            let msg = (error as? BridgeError)?.description ?? error.localizedDescription
            let output = "{\"error\":\"\(msg.replacingOccurrences(of: "\"", with: "\\\""))\"}\n"
            FileHandle.standardError.write(output.data(using: .utf8)!)
            Foundation.exit(1)
        }
    }
}
