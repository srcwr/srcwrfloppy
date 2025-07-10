#include "../../../srcwrtimer/extshared/src/extension.h"
#include "../../../srcwrtimer/extshared/src/coreident.hpp"


extern const sp_nativeinfo_t DeltaNatives[];


void MyExtension::OnHandleDestroy(HandleType_t type, void* object) {}
bool MyExtension::GetHandleApproxSize(HandleType_t type, void* object, unsigned int* size) { return false; }


bool Extension_OnLoad(char* error, size_t maxlength)
{
	sharesys->AddNatives(myself, DeltaNatives);
	return true;
}

void Extension_OnUnload()
{
}

void Extension_OnAllLoaded() {}

static cell_t N_SRCWRDELTA_AsyncSaveReplay(IPluginContext* ctx, const cell_t* params)
{
	char *replayfmt;
	(void)ctx->LocalToString(params[1], &replayfmt);
	int replaysubversion = params[2];

	if (0 != strcmp(replayfmt, "{SHAVITREPLAYFORMAT}{FINAL}") || replaysubversion != 0x09)
	{
		// TODO: Throw error
	}

	//int callback = params[3];
	int value = params[4];

	char *replayFolder;
	(void)ctx->LocalToString(params[5], &replayFolder);

	int style = params[6];
	int track = params[7];
	float time = params[8];
	int steamid = params[9];
	int preframes = params[10];
	//int playerrecording = params[11];
	int iSize = params[12];
	int postframes = params[13];
	int timestamp = params[14];
	//int fZoneOffset = params[15];
	bool saveCopy = params[16];
	bool saveWR = params[17];

	return 0; // native marked as void so return value doesn't matter...
}

extern const sp_nativeinfo_t DeltaNatives[] = {
	{"SRCWRDELTA_AsyncSaveReplay", N_SRCWRDELTA_AsyncSaveReplay},
	{NULL, NULL}
};
