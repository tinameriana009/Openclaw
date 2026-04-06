#pragma once

#include "Kismet/BlueprintFunctionLibrary.h"
#include "TelemetryBlueprintLibrary.generated.h"

class UObject;

UCLASS()
class RUNTIMETELEMETRY_API UTelemetryBlueprintLibrary : public UBlueprintFunctionLibrary
{
    GENERATED_BODY()

public:
    UFUNCTION(BlueprintCallable, Category = "Telemetry", meta = (WorldContext = "WorldContextObject"))
    static void RecordTelemetryEvent(UObject* WorldContextObject, const FString& EventName, const FString& Payload);
};
