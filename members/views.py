from drf_spectacular.utils import extend_schema
from rest_framework.decorators import api_view
from rest_framework.response import Response
import members.models as member_models
from members.serializers import MemberSerializer, MemberListSerializer, RelationshipCreateSerializer


@extend_schema(
    responses=MemberSerializer(many=True),
    description="Retrieve a list of all members with their details and medical certificate status."
)
@api_view(['GET'])
def get_members(request):
    members = member_models.Member.objects.all()
    serializer = MemberListSerializer(members, many=True)
    return Response(serializer.data)


@extend_schema(
    responses=MemberSerializer,
    description="Retrieve details of a specific member by their ID, including medical certificate status."
)
@api_view(['GET'])
def get_member(request, member_id):
    try:
        member = member_models.Member.objects.get(id=member_id)
    except member_models.Member.DoesNotExist:
        return Response({'error': 'Member not found'}, status=404)
    serializer = MemberSerializer(member)
    return Response(serializer.data)


@extend_schema(
    request=MemberSerializer,
    responses=MemberSerializer,
    description="Create a new member with the provided details."
)
@api_view(["POST"])
def create_member(request):
    serializer = MemberSerializer(data=request.data)
    if serializer.is_valid():
        serializer.save()
        return Response(serializer.data, status=201)
    return Response(serializer.errors, status=400)


@extend_schema(
    request=RelationshipCreateSerializer,
    responses=RelationshipCreateSerializer,
    description="Create a relationship between two members."
)
@api_view(['POST'])
def create_relationship(request):
    """
    Endpoint to link two members.
    Payload example: { "from_member": 2, "to_member": 1, "relation_type": "PARENT" }
    """
    serializer = RelationshipCreateSerializer(data=request.data)
    if serializer.is_valid():
        try:
            serializer.save()
            return Response(serializer.data, status=201)
        except Exception as e:
            return Response({"error": str(e)}, status=400)

    return Response(serializer.errors, status=400)